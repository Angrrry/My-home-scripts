use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Layout {
    pub code: String,
    pub variant: String,
    pub display_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserBus {
    uid: u32,
}

impl UserBus {
    pub fn new(uid: u32) -> Self {
        Self { uid }
    }

    fn runtime_dir(&self) -> String {
        format!("/run/user/{}", self.uid)
    }

    fn bus_address(&self) -> String {
        format!("unix:path=/run/user/{}/bus", self.uid)
    }

    fn busctl(&self) -> Result<Command, String> {
        let username = username_for_uid(self.uid)
            .ok_or_else(|| format!("could not find username for uid {}", self.uid))?;

        let mut command = Command::new("runuser");
        command.args([
            "-u",
            &username,
            "--",
            "env",
            &format!("XDG_RUNTIME_DIR={}", self.runtime_dir()),
            &format!("DBUS_SESSION_BUS_ADDRESS={}", self.bus_address()),
            "busctl",
            "--user",
        ]);
        command
            .env("XDG_RUNTIME_DIR", self.runtime_dir())
            .env("DBUS_SESSION_BUS_ADDRESS", self.bus_address());

        Ok(command)
    }
}

pub fn parse_layouts(output: &str) -> Vec<Layout> {
    let quoted = quoted_fields(output);
    quoted
        .chunks_exact(3)
        .map(|fields| Layout {
            code: fields[0].clone(),
            variant: fields[1].clone(),
            display_name: fields[2].clone(),
        })
        .collect()
}

pub fn find_layout_index(layouts: &[Layout], code: &str, variant: Option<&str>) -> Option<usize> {
    layouts.iter().position(|layout| {
        layout.code == code && variant.map_or(true, |variant| layout.variant == variant)
    })
}

pub fn get_layouts(bus: UserBus) -> Result<Vec<Layout>, String> {
    let output = bus
        .busctl()?
        .args([
            "call",
            "org.kde.keyboard",
            "/Layouts",
            "org.kde.KeyboardLayouts",
            "getLayoutsList",
        ])
        .output()
        .map_err(|error| format!("failed to run busctl: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "busctl getLayoutsList failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(parse_layouts(&String::from_utf8_lossy(&output.stdout)))
}

pub fn set_layout(bus: UserBus, index: usize) -> Result<(), String> {
    let status = bus
        .busctl()?
        .args([
            "call",
            "org.kde.keyboard",
            "/Layouts",
            "org.kde.KeyboardLayouts",
            "setLayout",
            "u",
            &index.to_string(),
        ])
        .status()
        .map_err(|error| format!("failed to run busctl: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("busctl setLayout failed with status {status}"))
    }
}

fn quoted_fields(input: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match (in_quotes, ch) {
            (false, '"') => {
                current.clear();
                in_quotes = true;
            }
            (true, '"') => {
                fields.push(current.clone());
                current.clear();
                in_quotes = false;
            }
            (true, '\\') => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            (true, ch) => current.push(ch),
            (false, _) => {}
        }
    }

    fields
}

fn username_for_uid(uid: u32) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let mut fields = line.split(':');
        let username = fields.next()?;
        fields.next()?;
        let line_uid = fields.next()?.parse::<u32>().ok()?;
        (line_uid == uid).then(|| username.to_string())
    })
}
