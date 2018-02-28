use std::fs::File;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};
use std::collections::HashMap;
use ParameterLevel;
use shared::{IntParameter, BoolParameter, ScriptInfo};

#[derive(Debug, Clone)]
pub struct XMLReader {
    targ_int_param: Vec<IntParameter>,
    targ_bool_param: Vec<BoolParameter>,
    hm_script: HashMap<String, String>,
}


impl XMLReader {
    pub fn new(file: String) -> Self {
        let file = File::open(file).unwrap();
        let file = BufReader::new(file);

        let parser = EventReader::new(file);

        let mut found_int_targ = false;
        let mut found_bool_targ = false;
        let mut found_script = false;


        let mut _hm_int_targ: HashMap<String, String> = HashMap::new();
        let mut _hm_bool_targ: HashMap<String, String> = HashMap::new();
        let mut _hm_script_info: HashMap<String, String> = HashMap::new();

        let mut _targ_int_p: Vec<IntParameter> = Vec::new();
        let mut _targ_bool_p: Vec<BoolParameter> = Vec::new();


        let mut tag = String::new();
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, .. }) => {
                    tag = name.to_string();
                    if name.to_string() == "int-parameter" {
                        found_int_targ = true;
                        found_bool_targ = false;
                        found_script = false;
                    } else if name.to_string() == "bool-parameter" {
                        found_int_targ = false;
                        found_bool_targ = true;
                        found_script = false;
                    } else if name.to_string() == "builder-script" {
                        found_int_targ = false;
                        found_bool_targ = false;
                        found_script = true;
                    }

                }
                Ok(XmlEvent::Characters(val)) => {
                    if found_int_targ == true {
                        _hm_int_targ.insert(tag.clone(), val.clone());
                    } else if found_bool_targ == true {
                        _hm_bool_targ.insert(tag.clone(), val.clone());
                    } else if found_script == true {
                        _hm_script_info.insert(tag.clone(), val.clone());
                    }
                }

                Ok(XmlEvent::EndElement { name }) => {

                    if name.to_string() == "int-parameter" {
                        found_int_targ = false;
                    } else if name.to_string() == "bool-parameter" {
                        found_bool_targ = false;
                    }


                    if name.to_string() == "int-parameter" {
                        let targ_int_param = IntParameter {
                            name: _hm_int_targ.get("name").unwrap().to_string(),
                            min: _hm_int_targ
                                .get("minimum")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                            max: _hm_int_targ
                                .get("maximum")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                            step: _hm_int_targ
                                .get("step")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                            default: _hm_int_targ
                                .get("default")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                            level: _hm_int_targ
                                .get("level")
                                .unwrap()
                                .to_string()
                                .parse()
                                .unwrap(),
                        };
                        _targ_int_p.push(targ_int_param);
                    }


                    if name.to_string() == "bool-parameter" {
                        let targ_bool_param = BoolParameter {
                            name: _hm_bool_targ.get("name").unwrap().to_string(),
                            true_val: _hm_bool_targ.get("true").unwrap().to_string(),
                            false_val: _hm_bool_targ.get("false").unwrap().to_string(),
                            default: _hm_bool_targ
                                .get("default")
                                .unwrap()
                                .to_string()
                                .parse::<bool>()
                                .unwrap(),
                            level: _hm_bool_targ
                                .get("level")
                                .unwrap()
                                .to_string()
                                .parse()
                                .unwrap(),
                        };
                        _targ_bool_p.push(targ_bool_param);
                    }

                }

                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }


        XMLReader {
            targ_int_param: _targ_int_p,
            targ_bool_param: _targ_bool_p,
            hm_script: _hm_script_info,
        }
    }

    /***********************************************************************************************************
    /// **
    /// Target Bool Params
    /// *
    	************************************************************************************************************/

    pub fn get_target_bool_params(&self) -> Vec<BoolParameter> {
        return self.targ_bool_param.clone();
    }


    /***********************************************************************************************************
    /// **
    /// Target Int Params
    /// *
    	************************************************************************************************************/

    pub fn get_target_int_hm_params(&self) -> Vec<IntParameter> {
        return self.targ_int_param.clone();
    }


    pub fn get_target_hm_service_level(&self) -> HashMap<String, ParameterLevel> {
        let mut hm_params: HashMap<String, ParameterLevel> = HashMap::new();

        for param in self.targ_bool_param.iter() {
            hm_params.insert(param.clone().name, param.clone().level);
        }

        for param in self.targ_int_param.iter() {
            hm_params.insert(param.clone().name, param.clone().level);
        }

        return hm_params;
    }

    /***********************************************************************************************************
    /// **
    /// Script Param
    /// *
    	************************************************************************************************************/

    pub fn get_script_info(&self) -> ScriptInfo {
        return ScriptInfo {
            name: self.hm_script.get("default").unwrap().to_string(),
            fulltag: self.hm_script.get("fulltag").unwrap().to_string(),
            envfile: self.hm_script.get("envfile").unwrap().to_string(),
        };
    }
}
