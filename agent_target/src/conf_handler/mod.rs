use std::fs;
use std::error::Error;
use std::io::prelude::*;
use std::fs::{DirEntry, File, OpenOptions};
use std::path::Path;
use std::io::BufReader;


pub struct ConfWriter {
    filename: String,
    text_lines: Vec<String>,
}


impl ConfWriter {
    pub fn new(file: String) -> Self {
        ConfWriter {
            filename: file,
            text_lines: Vec::new(),
        }

    }

    pub fn push_line(&mut self, line: String) {
        self.text_lines.push(line);
    }

    pub fn flush_write(&self) {
        let path = Path::new(self.filename.as_str());
        let display = path.display();

        // Open a file in write-only mode, returns `io::Result<File>`
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();

        let mut str_2_write = String::new();
        for line in self.text_lines.iter() {
            str_2_write += line;
        }

        match file.write_all(str_2_write.as_bytes()) {
            Err(why) => panic!("couldn't write to {}: {}", display, why.description()),
            Ok(_) => println!("successfully wrote to {}", display),
        }
    }
}


pub fn search_and_write(dir: String, param_2_find: String, param_value: String) {

    let mut found = false;
    let mut val_2_replace = String::new();
    let mut f_path = String::new();

    'outer: for entry in fs::read_dir(dir.clone()).expect("The directory does not exist") {
        let entry = entry.expect("I couldn't read something inside the directory");


        if !entry.path().is_dir() {
            f_path = entry.path().to_str().unwrap().to_string();
            let mut file = File::open(entry.path()).unwrap();

            let reader = BufReader::new(file);
            for line in reader.lines() {
                let l = line.unwrap();
                for word in l.clone().split_whitespace() {
                    if found == true {
                        val_2_replace = l;
                        break 'outer;
                    }
                    if word == param_2_find {
                        found = true;
                    }

                }
            }

        }
    }

    if found == true {
        let mut file = File::open(f_path.clone()).unwrap();
        let mut contents = String::new();

        file.read_to_string(&mut contents);
        let line_2_replace = format!("{} {}", param_2_find, param_value);
        let new_content = contents.replace(&*val_2_replace, &*line_2_replace);

        let mut updated_file = File::create(f_path).unwrap();
        updated_file.write_all(new_content.as_bytes()).unwrap();

    } else {
        panic!("Parameter {} not found in {}", param_2_find, dir);

    }
}
