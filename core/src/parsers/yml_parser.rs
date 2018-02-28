use std::fs::File;
use yaml_rust::yaml;
use std::io::Read;
#[derive(Debug, Clone)]
pub struct YMLReader {
    yml_string: String,
}

impl YMLReader {
    pub fn new(filename: String) -> Self {
        let mut f = File::open(&filename).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();


        let docs = yaml::YamlLoader::load_from_str(&s).unwrap();

        let doc = &docs[0];

        YMLReader { yml_string: s }

    }


    pub fn get_target_addresses(&self) -> Vec<String> {
        let docs = yaml::YamlLoader::load_from_str(&self.yml_string).unwrap();
        let doc = &docs[0];

        let mut ret_val: Vec<String> = Vec::new();

        let mut index = 0;
        loop {
            let target_str = format!("agent_target_{}", index.to_string());
            if doc["services"][target_str.as_str()]["networks"]["app_net"]["ipv4_address"]
                .is_badvalue()
            {
                break;
            }
            let val_2_push = format!(
                "{}:{}",
                doc["services"][target_str.as_str()]["networks"]["app_net"]["ipv4_address"]
                    .as_str()
                    .unwrap(),
                doc["services"][target_str.as_str()]["expose"][0]
                    .as_i64()
                    .unwrap()
            );

            ret_val.push(val_2_push);

            index += 1;
        }

        return ret_val;
    }


    pub fn get_bench_addresses(&self) -> Vec<String> {
        let docs = yaml::YamlLoader::load_from_str(&self.yml_string).unwrap();
        let doc = &docs[0];

        let mut ret_val: Vec<String> = Vec::new();

        let mut index = 0;
        loop {
            let bench_str = format!("agent_bench_{}", index.to_string());
            index += 1;

            if doc["services"][bench_str.as_str()]["networks"]["app_net"]["ipv4_address"]
                .is_badvalue()
            {
                break;
            }
            let val_2_push = format!(
                "{}:{}",
                doc["services"][bench_str.as_str()]["networks"]["app_net"]["ipv4_address"]
                    .as_str()
                    .unwrap(),
                doc["services"][bench_str.as_str()]["expose"][0]
                    .as_i64()
                    .unwrap()
            );

            ret_val.push(val_2_push);

        }

        return ret_val;
    }

    pub fn get_num_targets(&self) -> usize {
        let docs = yaml::YamlLoader::load_from_str(&self.yml_string).unwrap();
        let doc = &docs[0];

        let mut index = 0;
        loop {
            let target_str = format!("agent_target_{}", index.to_string());
            index += 1;

            if doc["services"][target_str.as_str()]["networks"]["app_net"]["ipv4_address"]
                .is_badvalue()
            {
                break;
            }

        }

        return index;
    }

    pub fn get_influx_address(&self) -> String {
        let docs = yaml::YamlLoader::load_from_str(&self.yml_string).unwrap();
        let doc = &docs[0];


        if doc["services"]["influxdb"]["networks"]["app_net"]["ipv4_address"].is_badvalue() {
            panic!("Error! Network configuration of container 'influxdb' not found");
        } else {
            return doc["services"]["influxdb"]["networks"]["app_net"]["ipv4_address"]
                .as_str()
                .unwrap()
                .to_string();
        }

    }
}
