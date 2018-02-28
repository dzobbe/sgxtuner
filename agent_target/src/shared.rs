use ParameterLevel;


#[derive(Debug, Clone)]
pub struct ScriptInfo {
    pub name: String,
    pub fulltag: String,
    pub envfile: String,
}

#[derive(Debug, Clone)]
pub struct IntParameter {
    pub name: String,
    pub min: usize,
    pub max: usize,
    pub step: usize,
    pub default: usize,
    pub level: ParameterLevel,
}

#[derive(Debug, Clone)]
pub struct BoolParameter {
    pub name: String,
    pub true_val: String,
    pub false_val: String,
    pub default: bool,
    pub level: ParameterLevel,
}
