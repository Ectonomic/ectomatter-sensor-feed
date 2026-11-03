//! The Sensor Feed plugin: every sensor on this computer, found and named by the
//! Sensors library (crates/ectomatter-sensors), and read for whatever shows
//! them. It lists them as it starts and every half minute after, telling
//! Ectomatter the list whenever it changes (readings/list), and reads those
//! it's asked to watch (readings/watch) once a second, telling each reading
//! as it comes (readings/changed).
//!
//! Everything is read through Ectomatter: the files its manifest names, read
//! only (the system interface's `read`, `entries` and `resolve`), and NVIDIA
//! cards through NVIDIA's driver, which Ectomatter loads itself (`nvidia`).

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};

use ectomatter_sensors::files::{self, Files};
use ectomatter_sensors::list::{self, Catalog, Nv, Nvidia, Probe, Val};
use ectomatter_sensors::read::Reader;
use serde_json::{json, Value};

wit_bindgen::generate!({ path: "../../../wit", world: "core" });

use ectomatter::core::{host, system};

/// The timers: reading what's watched, and listing again.
const READ: u32 = 1;
const LIST: u32 = 2;

/// The files, as Ectomatter lends them.
struct Lent;

impl Files for Lent {
    fn read(&self, path: &Path) -> io::Result<String> {
        system::read(&path.to_string_lossy()).map_err(io::Error::other)
    }
    fn entries(&self, path: &Path) -> io::Result<Vec<String>> {
        system::entries(&path.to_string_lossy()).map_err(io::Error::other)
    }
    fn resolve(&self, path: &Path) -> io::Result<PathBuf> {
        system::resolve(&path.to_string_lossy()).map(PathBuf::from).map_err(io::Error::other)
    }
}

/// NVIDIA's driver, as Ectomatter reaches it.
struct Driver;

fn word(n: Nv) -> &'static str {
    match n {
        Nv::Temp => "temp",
        Nv::Load => "load",
        Nv::Clock => "clock",
        Nv::MemClock => "mem-clock",
        Nv::Vram => "vram",
        Nv::Power => "power",
        Nv::Fan => "fan",
    }
}

impl Nvidia for Driver {
    fn probe(&mut self, pci: &str) -> Option<Probe> {
        let v: Value = serde_json::from_str(&system::nvidia(pci, "probe").ok()?).ok()?;
        let all = [Nv::Temp, Nv::Load, Nv::Clock, Nv::MemClock, Nv::Vram, Nv::Power, Nv::Fan];
        let has = v["has"].as_array()?.iter().filter_map(|w| all.into_iter().find(|n| w.as_str() == Some(word(*n)))).collect();
        Some(Probe { has, vram: v["vram"].as_f64().unwrap_or(0.0) })
    }
    fn read(&mut self, pci: &str, what: Nv) -> Option<Val> {
        serde_json::from_str(&system::nvidia(pci, word(what)).ok()?).ok()
    }
}

#[derive(Default)]
struct State {
    reader: Option<Reader>,
    catalog: Catalog,
    /// the sensors Ectomatter wants read, by id
    watch: Vec<String>,
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

impl State {
    /// The computer's sensors listed again, and told where the list has changed.
    fn list(&mut self) {
        let Some(reader) = self.reader.as_mut() else { return };
        let cat = list::catalog(Path::new("/"), reader.nvidia());
        if cat != self.catalog {
            self.catalog = cat;
            // (no readings of its own beside them: the sensors, as a catalog)
            host::notify("readings/list", &json!({ "readings": [], "catalog": self.catalog }).to_string());
        }
    }

    /// The sensors watched, read now and told.
    fn read(&mut self) {
        let Some(reader) = self.reader.as_mut() else { return };
        if self.watch.is_empty() {
            return;
        }
        let want: Vec<&list::Sensor> = self.catalog.devices.iter().flat_map(|d| d.sensors.iter()).filter(|s| self.watch.contains(&s.id)).collect();
        let values = reader.read(&want);
        host::notify("readings/changed", &json!({ "values": values }).to_string());
    }
}

fn answer(method: &str, p: &Value) -> Result<Value, String> {
    match method {
        "initialize" => {
            let met = ectomatter_protocol::meet(p, "Sensor Feed", env!("CARGO_PKG_VERSION"))?;
            files::reach_through(Box::new(Lent));
            S.with(|s| {
                let mut s = s.borrow_mut();
                s.reader = Some(Reader::new("/", Box::new(Driver)));
                s.list();
            });
            host::after(1000, READ);
            host::after(30_000, LIST);
            Ok(met)
        }
        // what Ectomatter wants read from now on; read at once, so it isn't a second late
        "readings/watch" => {
            S.with(|s| {
                let mut s = s.borrow_mut();
                s.watch = p["ids"].as_array().into_iter().flatten().filter_map(|i| i.as_str().map(String::from)).collect();
                s.read();
            });
            Ok(Value::Null)
        }
        "shutdown" => Ok(Value::Null),
        _ => Err(format!("The Sensor Feed doesn’t answer {method}.")),
    }
}

struct Plugin;

impl Guest for Plugin {
    fn call(method: String, params: String) -> Result<String, String> {
        let p: Value = if params.is_empty() { Value::Null } else { serde_json::from_str(&params).map_err(|e| format!("{method}: {e}"))? };
        answer(&method, &p).map(|v| v.to_string())
    }

    fn timer(tag: u32) {
        S.with(|s| {
            let mut s = s.borrow_mut();
            match tag {
                READ => {
                    s.read();
                    host::after(1000, READ);
                }
                LIST => {
                    s.list();
                    host::after(30_000, LIST);
                }
                _ => {}
            }
        });
    }
}

export!(Plugin);
