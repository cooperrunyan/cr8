use log::{debug, info, log_enabled, Level};
use path_absolutize::Absolutize;
use std::collections::HashMap;
use std::env;

use super::Compiler;

impl Compiler {
    pub(crate) fn debug(&self) {
        self.debug_labels();
        self.debug_files();
        self.debug_statics();

        if log_enabled!(Level::Debug) {
            self.debug_macros();
            self.debug_bin();
        }
    }

    fn debug_files(&self) {
        info!("===== Files Used: =====");
        let mut pwd = env::current_dir().unwrap().display().to_string();
        pwd.push('/');
        for file in self.files.iter() {
            info!(
                "  - {}",
                file.absolutize()
                    .unwrap()
                    .display()
                    .to_string()
                    .replace(&pwd, "")
            );
        }
        info!("");
    }

    fn debug_statics(&self) {
        info!("======== Statics: ========");
        let mut sorted: Vec<_> = self.statics.iter().collect();
        sorted.sort_by(|a, b| a.0.partial_cmp(b.0).unwrap());
        for (k, v) in sorted {
            info!("  - {}: {:#06X}", k, v);
        }
        info!("");
    }

    fn debug_macros(&self) {
        debug!("===== Macros Declared: =====");
        for (name, mac) in self.macros.iter() {
            let mut args = String::new();
            for (i, arg) in mac.args.iter().enumerate() {
                args.push_str(&format!("{arg}"));
                if i != mac.args.len() - 1 {
                    args.push_str(&format!(", "));
                }
            }
            debug!("  {name} [{}]:", args);
            for inst in mac.body.iter() {
                debug!("    {}", inst);
            }
            debug!("");
        }
        debug!("");
    }

    fn debug_labels(&self) {
        info!("===== Labels: =====");
        let mut sorted: Vec<_> = self.labels.iter().collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

        for (name, location) in sorted {
            info!("  - {name}: {:?}", location);
        }
        info!("");
    }

    fn debug_bin(&self) {
        let mut label_reverse_lookup: HashMap<usize, &str> = HashMap::new();

        for (name, location) in self.labels.iter() {
            label_reverse_lookup.insert(*location, &name);
        }

        debug!("===== Binary: =====");
        for (location, byte) in self.bin.iter().enumerate() {
            if let Some(label) = label_reverse_lookup.get(&location) {
                debug!("");
                debug!("{}:", label);
                if let Some(marker) = self.markers.get(&location) {
                    debug!("  {location:04x}: {byte:02x}    {marker}");
                } else {
                    debug!("  {location:04x}: {byte:02x}");
                }
            } else {
                if let Some(marker) = self.markers.get(&location) {
                    debug!("  {location:04x}: {byte:02x}    {marker}");
                } else {
                    debug!("  {location:04x}: {byte:02x}");
                }
            }
        }
        debug!("");
    }
}
