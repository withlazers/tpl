use gtmpl;
use gtmpl::Value;
use serde_any::Format;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug)]
struct TemplateObject {
    values: Value,
}

impl TemplateObject {
    pub fn new() -> Self {
        Self {
            values: Value::Object(HashMap::new()),
        }
    }

    pub fn merge(&mut self, value: Value) {
        self.values.merge(value)
    }

    pub fn set(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut stage =
            serde_any::from_str(value, Format::Yaml).map_err(|_| {
                format!("Error parsing set option {}={}", key, value)
            })?;
        for seg in key.rsplit(".").map(str::trim) {
            let mut map = HashMap::new();
            map.insert(seg.to_string(), stage);
            stage = map.into();
        }
        self.merge(stage);
        Ok(())
    }

    pub fn dump_var(&self, format: Format) -> Result<(), Box<dyn Error>> {
        serde_any::to_writer_pretty(io::stdout(), &self.values, format)
            .map_err(|_| format!("Error writing template"))?;
        println!("");
        Ok(())
    }
    pub fn template<R: Read>(
        self,
        mut reader: R,
    ) -> Result<(), Box<dyn Error>> {
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer)?;
        let template = String::from_utf8(buffer)?;
        println!("{}", gtmpl::template(&template, self.values)?);
        Ok(())
    }
}

trait Merge {
    fn merge(&mut self, other: Self);
}

impl Merge for HashMap<String, Value> {
    fn merge(&mut self, other: Self) {
        for (k, v) in other {
            self.entry(k.to_string()).or_insert(Value::NoValue).merge(v)
        }
    }
}

impl Merge for Value {
    fn merge(&mut self, other: Self) {
        use Value::*;
        match (self, other) {
            (Object(a), Object(b)) => a.merge(b),
            (Map(a), Map(b)) => a.merge(b),
            (_, Nil) => (),
            (a, b) => *a = b,
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt()]
struct Opt {
    #[structopt(name = "TEMPLATE", parse(from_os_str))]
    template: Option<PathBuf>,
    #[structopt(short, long, parse(from_os_str), number_of_values = 1)]
    values: Vec<PathBuf>,

    #[structopt(short, long, parse(try_from_str = parse_key_val), number_of_values = 1)]
    set: Vec<(String, String)>,

    #[structopt(short, long)]
    dumpvar: Option<serde_any::Format>,
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error>>
where
    T: std::str::FromStr,
    T::Err: Error + 'static,
    U: std::str::FromStr,
    U::Err: Error + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let mut template_object = TemplateObject::new();

    for value_path in opt.values {
        let values: Value = serde_any::from_file(&value_path)
            .map_err(|_| format!("Cannot find `{}`", &value_path.display()))?;

        template_object.merge(values);
    }

    for (k, v) in opt.set {
        template_object.set(&k, &v)?;
    }

    if let Some(format) = opt.dumpvar {
        template_object.dump_var(format)?;
    } else if let Some(template) = opt.template {
        template_object.template(File::open(template)?)?;
    } else {
        template_object.template(io::stdin())?;
    }

    Ok(())
}
