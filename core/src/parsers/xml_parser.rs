use std::fs::File;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};
use std::collections::HashMap;
use EnergyType;
use ExecutionType;
use CoolingSchedule;
use SolverVersion;
use shared::{IntParameter, BoolParameter, TunerParameter, ScriptInfo};

#[derive(Debug, Clone)]
pub struct XMLReader {
    hm_tuner: HashMap<String, String>,
    targ_int_param: Vec<IntParameter>,
    targ_bool_param: Vec<BoolParameter>,
    hm_script: HashMap<String, String>,
}


impl XMLReader {
    pub fn new(file: String) -> Self {
        let file = File::open(file).unwrap();
        let file = BufReader::new(file);

        let parser = EventReader::new(file);

        let mut found_tuner = false;
        let mut found_int_targ = false;
        let mut found_bool_targ = false;
        let mut found_script = false;


        let mut _hm_tuner: HashMap<String, String> = HashMap::new();
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
                    if name.to_string() == "tuner-params" {
                        found_tuner = true;
                        found_int_targ = false;
                        found_bool_targ = false;
                        found_script = false;
                    } else if name.to_string() == "int-parameter" {
                        found_tuner = false;
                        found_int_targ = true;
                        found_bool_targ = false;
                        found_script = false;
                    } else if name.to_string() == "bool-parameter" {
                        found_tuner = false;
                        found_int_targ = false;
                        found_bool_targ = true;
                        found_script = false;
                    } else if name.to_string() == "builder-script" {
                        found_tuner = false;
                        found_int_targ = false;
                        found_bool_targ = false;
                        found_script = true;
                    }

                }
                Ok(XmlEvent::Characters(val)) => {
                    if found_tuner == true {
                        _hm_tuner.insert(tag.clone(), val.clone());
                    } else if found_int_targ == true {
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
                        let mut targ_int_param = IntParameter {
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
                        let mut targ_bool_param = BoolParameter {
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
            hm_tuner: _hm_tuner,
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

    pub fn get_target_int_params(&self) -> Vec<IntParameter> {
        return self.targ_int_param.clone();
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




    /***********************************************************************************************************
    /// **
    /// Tuner Parameters
    /// *
     ************************************************************************************************************/
    pub fn get_tuner_params(&self) -> TunerParameter {
        return TunerParameter {
            max_step: self.ann_max_steps(),
            num_iter: self.ann_num_iter(),
            min_temp: self.ann_min_temp(),
            max_temp: self.ann_max_temp(),
            energy: self.ann_energy(),
            cooling: self.ann_cooling(),
            version: self.ann_version(),
        };
    }

    fn ann_max_steps(&self) -> usize {
        return self.hm_tuner
            .get("max_step")
            .unwrap()
            .to_string()
            .parse()
            .unwrap();
    }
    fn ann_num_iter(&self) -> u8 {
        return self.hm_tuner
            .get("num_iter")
            .unwrap()
            .to_string()
            .parse()
            .unwrap();
    }
    fn ann_min_temp(&self) -> Option<f64> {
        match self.hm_tuner.get("min_temp") {
            Some(val) => return Some(val.to_string().parse::<f64>().unwrap()),
            None => return None,
        };
    }
    fn ann_max_temp(&self) -> Option<f64> {
        match self.hm_tuner.get("max_temp") {
            Some(val) => return Some(val.to_string().parse::<f64>().unwrap()),
            None => return None,
        };
    }

    fn ann_energy(&self) -> EnergyType {
        let energy_type: EnergyType = self.hm_tuner
            .get("energy")
            .unwrap()
            .to_string()
            .parse()
            .unwrap();
        return energy_type;
    }
    fn ann_cooling(&self) -> CoolingSchedule {
        let cooling_schedule: CoolingSchedule = self.hm_tuner
            .get("cooling")
            .unwrap()
            .to_string()
            .parse()
            .unwrap();
        return cooling_schedule;
    }

    fn ann_version(&self) -> SolverVersion {
        let solver_version: SolverVersion = self.hm_tuner
            .get("version")
            .unwrap()
            .to_string()
            .parse()
            .unwrap();
        return solver_version;
    }
}
