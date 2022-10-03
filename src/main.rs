use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;
use simple_logger::SimpleLogger;
use std::fs::File;
use std::io::Write;
use std::{fs, process};

fn quit(error_message: &str) -> ! {
    log::error!("{}", error_message);
    process::exit(1)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    branch_coverage_level: f32,
    line_coverage_level: f32,
    upgrade_config_after_check: bool,
    coverage_level_file_path: String,
}

fn read_config() -> Config {
    match fs::read_to_string("oreiller.toml") {
        Ok(config_file) => {
            match toml::from_str(&config_file) {
                Ok(config_file) => {
                    let config: Config = config_file;
                    return config;
                }
                Err(error) => {
                    println!("{}", error);
                    quit("Failed to parse config");
                }
            };
        }
        Err(error) => quit("Failed to read config file"),
    };
    quit("Unexpected")
}

// <coverage lines-valid="4067" lines-covered="3811" line-rate="0.937" branchs-valid
// ="356" branchs-covered="275" branch-rate="0.7724" timestamp="1658312473271" compl

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
struct Cobertura {
    branch_rate: f32,
    line_rate: f32,
}

fn read_coverages_levels(config: &Config) -> Cobertura {
    match fs::read_to_string(config.coverage_level_file_path.clone()) {
        Ok(cobertura) => {
            let cobertura = str::replace(&cobertura, "&", "&amp;");
            match from_str(&cobertura) {
                Ok(cobertura) => return cobertura,
                Err(error) => {
                    println!("{}", error);
                    quit("Failed to parse cobertura file")
                }
            };
        }
        Err(error) => {
            println!("{}", error);
            quit("Failed to read cobertura result file")
        }
    };
}

fn has_coverage_decreased(required: &Config, coverage_level: &Cobertura) -> bool {
    let mut branch_coverage_level_decreased = false;
    if required.branch_coverage_level > coverage_level.branch_rate {
        branch_coverage_level_decreased = true;

        log::warn!(
            "insuffisant branche level statement: {} > {}",
            required.branch_coverage_level,
            coverage_level.branch_rate,
        );
    }
    let mut line_coverage_level_decreased = false;
    if required.line_coverage_level > coverage_level.line_rate {
        line_coverage_level_decreased = true;
        log::warn!(
            "insuffisant line level statement: {} > {}",
            required.line_coverage_level,
            coverage_level.line_rate,
        );
    }

    return branch_coverage_level_decreased || line_coverage_level_decreased;
}

fn quit_if_coverage_decreased(required: &Config, coverage_level: &Cobertura) {
    if has_coverage_decreased(required, coverage_level) {
        let mut let_coverage_decrease = false;
        match std::env::var("OREILLER_LET_COVERAGE_DECREASE") {
            Ok(variable) => {
                if variable == "true" {
                    let_coverage_decrease = true;
                    log::info!("You setted $OREILLER_LET_COVERAGE_DECREASE, not a fatal error.");
                }
            }
            Err(_) => {}
        }
        if !let_coverage_decrease {
            log::info!("You can set OREILLER_LET_COVERAGE_DECREASE='true' to allow failure");
            quit("Your coverage is too low");
        }
    } else {
        log::info!("Your coverage is ok");
    }
}

fn has_coverage_increased(required: &Config, coverage_level: &Cobertura) -> bool {
    let mut branch_coverage_level_increased = false;
    if required.branch_coverage_level < coverage_level.branch_rate {
        branch_coverage_level_increased = true;
        log::info!(
            "Branch coverage level increased: from {} to {}",
            required.branch_coverage_level,
            coverage_level.branch_rate,
        );
    }
    let mut line_coverage_level_increased = false;
    if required.line_coverage_level < coverage_level.line_rate {
        line_coverage_level_increased = true;
        log::info!(
            "Line coverage level increased: from {} to {}",
            required.line_coverage_level,
            coverage_level.line_rate,
        );
    }

    branch_coverage_level_increased || line_coverage_level_increased
}

fn get_config_with_new_requiredlevels(required: &Config, coverage_level: &Cobertura) -> Config {
    let mut config = required.clone();
    config.line_coverage_level = coverage_level.line_rate;
    config.branch_coverage_level = coverage_level.branch_rate;
    config
}

fn write_new_coverage_level(config: Config) {
    let config_path = "oreiller.toml";
    let config = toml::to_string(&config).unwrap();
    match File::create(config_path) {
        Ok(mut file) => match file.write_all(config.as_bytes()) {
            Ok(_) => {
                log::info!(
                    "New coverage level(s) were written into config file {}",
                    &config_path,
                );
            }
            Err(error) => {
                log::error!("{}", error);
                quit("Failed to write new config file")
            }
        },
        Err(error) => {
            log::error!("{}", error);
            quit("Failed to write new config file")
        }
    }
}

fn write_new_coverage_level_if_enabled(config: Config) {
    if config.upgrade_config_after_check {
        write_new_coverage_level(config)
    }
}

fn write_new_coverage_level_if_required(config: Config, coverage_level: &Cobertura) {
    if has_coverage_increased(&config, coverage_level) {
        let config = get_config_with_new_requiredlevels(&config, coverage_level);
        write_new_coverage_level_if_enabled(config)
    }
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let config = read_config();
    let coverage_level = read_coverages_levels(&config);
    quit_if_coverage_decreased(&config, &coverage_level);
    write_new_coverage_level_if_required(config, &coverage_level);
}
