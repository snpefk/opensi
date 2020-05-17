extern crate quick_xml;
extern crate serde;

use std::fs::File;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::path::Path;
use PartialEq;

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use quick_xml::de::{from_reader, from_str, DeError};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Package {
    pub id: String,
    pub name: Option<String>,
    pub version: String,
    pub date: Option<String>,
    pub difficulty: Option<u8>,
    pub language: Option<String>,
    pub logo: Option<String>,
    pub publisher: Option<String>,
    pub restriciton: Option<String>,
    pub rounds: Rounds,
    pub tags: Option<Vec<String>>,
    pub info: Info,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Info {
    pub comments: Option<String>,
    pub extension: Option<String>,
    pub authors: Authors,
    pub sources: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Authors {
    #[serde(rename = "author", default)]
    pub authors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Rounds {
    #[serde(rename = "round", default)]
    pub rounds: Vec<Round>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Round {
    pub name: String,
    #[serde(rename = "type", default)]
    pub variant: Option<String>,
    pub info: Option<Info>,
    pub themes: Themes,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Themes {
    #[serde(rename = "theme", default)]
    pub themes: Vec<Theme>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Theme {
    pub name: String,
    pub questions: Questions,
    pub info: Option<Info>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Questions {
    #[serde(rename = "question", default)]
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Question {
    pub price: usize,
    pub scenario: Scenario,
    pub right: Right,
    pub wrong: Option<Wrong>,
    #[serde(rename = "type", default)]
    pub variant: Option<Variant>,
    pub info: Option<Info>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Variant {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Scenario {
    #[serde(rename = "atom", default)]
    pub atoms: Vec<Atom>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Right {
    #[serde(rename = "answer", default)]
    pub answers: Vec<Answer>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Wrong {
    #[serde(rename = "answer", default)]
    pub answers: Vec<Answer>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Answer {
    #[serde(rename = "$value")]
    pub body: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Atom {
    pub time: Option<f64>,
    #[serde(rename = "type", default)]
    pub variant: Option<String>,
    #[serde(rename = "$value")]
    pub body: Option<String>,
}

const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ');

impl Package {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Package, std::io::Error> {
        let package_file = File::open(path)?;
        let mut zip = zip::ZipArchive::new(package_file)?;
        let mut xml = zip.by_name("content.xml")?;
        let mut contents = String::new();
        xml.read_to_string(&mut contents).unwrap();

        Package::parse(&contents).map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
    }

    fn parse(xml: &str) -> Result<Package, DeError> {
        let package: Package = from_str(xml)?;
        Result::Ok(package)
    }

    pub fn open_with_extraction<P: AsRef<Path>>(path: P) -> Result<Package, std::io::Error> {
        let package_name = path.as_ref().file_name().unwrap().to_str().unwrap();
        let package_file = File::open(&path)?;
        let mut zip = zip::ZipArchive::new(package_file)?;

        let xml_path = Self::extract_package(package_name, &mut zip)?;
        let xml = std::fs::File::open(xml_path)?;
        let reader = std::io::BufReader::new(xml);
        from_reader(reader).map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
    }

    /// Extract package internals into `/tmp/{package_name}/`. Return `PathBuf`
    /// to `content.xml`.
    ///
    /// # Arguments
    /// * `package_name` - package name to which the package will be extracted
    /// * `zip` - unpacked archive
    fn extract_package(
        package_name: &str,
        zip: &mut zip::ZipArchive<File>,
    ) -> Result<std::path::PathBuf, std::io::Error> {
        let tmp = std::env::temp_dir().join(package_name);

        for i in 0..zip.len() {
            let mut zipfile = zip.by_index(i)?;
            let mut zipfile_path = zipfile.sanitized_name();
            let encoded_name = Self::encode_file_name(&zipfile_path);

            if encoded_name.starts_with("@") {
                zipfile_path.set_file_name(&encoded_name[1..]);
            } else {
                zipfile_path.set_file_name(encoded_name)
            };

            if let Some(parent) = zipfile_path.parent() {
                let parent_path = tmp.join(parent);
                if !parent.exists() {
                    std::fs::create_dir_all(&parent_path)?;
                }
            }

            let path = &tmp.join(zipfile_path);
            let mut fsfile = std::fs::File::create(&path)?;
            std::io::copy(&mut zipfile, &mut fsfile)?;
        }
        Ok(tmp.join("content.xml"))
    }

    fn encode_file_name(path: &std::path::PathBuf) -> String {
        return path
            .file_name()
            .unwrap()
            .to_str()
            .map(|filename| utf8_percent_encode(filename, FRAGMENT))
            .unwrap()
            .to_string();
    }
}
