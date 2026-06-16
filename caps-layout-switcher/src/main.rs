use caps_layout_switcher::evdev::{
    Device, EV_KEY, EV_SYN, KEY_DOWN, KEY_UP, candidate_keyboard_devices,
};
use caps_layout_switcher::gestures::{CapsEvent, CapsGesture, GestureDetector};
use caps_layout_switcher::kde::{UserBus, find_layout_index, get_layouts, set_layout};
use caps_layout_switcher::uinput::VirtualKeyboard;
use std::env;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_USER_UID: u32 = 1000;
const DEFAULT_DOUBLE_TAP_MS: u64 = 250;
const DEFAULT_HOLD_MS: u64 = 400;

#[derive(Debug)]
struct Config {
    device_path: Option<PathBuf>,
    user_uid: u32,
    double_tap_timeout: Duration,
    hold_timeout: Duration,
    list_devices: bool,
}

#[derive(Clone, Copy, Debug)]
struct LayoutTargets {
    english: usize,
    belarusian: usize,
    russian: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("caps-layout-switcher: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.list_devices {
        for path in candidate_keyboard_devices() {
            println!("{}", path.display());
        }
        return Ok(());
    }

    let bus = UserBus::new(config.user_uid);
    let targets = resolve_layout_targets(bus)?;
    let device_path = select_device(config.device_path)?;
    eprintln!(
        "caps-layout-switcher: listening on {}",
        device_path.display()
    );

    let mut device = Device::open(&device_path)
        .map_err(|error| format!("failed to open {}: {error}", device_path.display()))?;
    let mut virtual_keyboard = VirtualKeyboard::create().map_err(|error| {
        format!("failed to create /dev/uinput virtual keyboard: {error}; try: sudo modprobe uinput")
    })?;
    device
        .grab()
        .map_err(|error| format!("failed to grab {}: {error}", device_path.display()))?;

    let mut detector = GestureDetector::new(config.double_tap_timeout, config.hold_timeout);

    loop {
        if let Some(gesture) = detector.poll(now()) {
            apply_gesture(bus, targets, gesture)?;
        }

        let event = device
            .read_event_timeout(Duration::from_millis(25))
            .map_err(|error| format!("failed to read input event: {error}"))?;

        let Some(event) = event else {
            continue;
        };

        if !event.is_caps_lock() && (event.event_type == EV_KEY || event.event_type == EV_SYN) {
            virtual_keyboard
                .write_event(&event)
                .map_err(|error| format!("failed to write virtual keyboard event: {error}"))?;
        }

        if !event.is_caps_lock() {
            continue;
        }

        let caps_event = match event.value {
            KEY_DOWN => CapsEvent::Down,
            KEY_UP => CapsEvent::Up,
            _ => continue,
        };

        if let Some(gesture) = detector.handle(caps_event, now()) {
            apply_gesture(bus, targets, gesture)?;
        }
    }
}

impl Config {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut config = Self {
            device_path: None,
            user_uid: env::var("SUDO_UID")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_USER_UID),
            double_tap_timeout: Duration::from_millis(DEFAULT_DOUBLE_TAP_MS),
            hold_timeout: Duration::from_millis(DEFAULT_HOLD_MS),
            list_devices: false,
        };

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--device" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--device requires a path".to_string())?;
                    config.device_path = Some(PathBuf::from(value));
                }
                "--user-uid" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--user-uid requires a numeric uid".to_string())?;
                    config.user_uid = value
                        .parse()
                        .map_err(|_| format!("invalid --user-uid value: {value}"))?;
                }
                "--double-ms" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--double-ms requires milliseconds".to_string())?;
                    let millis = value
                        .parse()
                        .map_err(|_| format!("invalid --double-ms value: {value}"))?;
                    config.double_tap_timeout = Duration::from_millis(millis);
                }
                "--hold-ms" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--hold-ms requires milliseconds".to_string())?;
                    let millis = value
                        .parse()
                        .map_err(|_| format!("invalid --hold-ms value: {value}"))?;
                    config.hold_timeout = Duration::from_millis(millis);
                }
                "--list-devices" => config.list_devices = true,
                "--help" | "-h" => {
                    print_help();
                    process::exit(0);
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }

        Ok(config)
    }
}

fn print_help() {
    println!(
        "Usage: caps-layout-switcher [--device PATH] [--user-uid UID] [--double-ms N] [--hold-ms N]\n\
         \n\
         CapsLock gestures:\n\
           single tap  -> English (us)\n\
           double tap  -> Belarusian intl (by:intl, falling back to by)\n\
           hold        -> Russian (ru)\n\
         \n\
         Options:\n\
           --list-devices     print candidate /dev/input devices\n\
           --device PATH      read a specific evdev device\n\
           --user-uid UID     KDE session user uid, default 1000 or SUDO_UID\n\
           --double-ms N      double tap window, default 250\n\
           --hold-ms N        hold threshold, default 400"
    );
}

fn select_device(configured: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = configured {
        return Ok(path);
    }

    let candidates = candidate_keyboard_devices();
    match candidates.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err("no candidate keyboard devices found; pass --device /dev/input/eventX".into()),
        many => Err(format!(
            "multiple candidate keyboard devices found: {}; pass --device PATH",
            many.iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn resolve_layout_targets(bus: UserBus) -> Result<LayoutTargets, String> {
    let layouts = get_layouts(bus)?;
    let english = find_layout_index(&layouts, "us", None)
        .ok_or_else(|| "KDE layout not found: us".to_string())?;
    let belarusian = find_layout_index(&layouts, "by", Some("intl"))
        .or_else(|| find_layout_index(&layouts, "by", None))
        .ok_or_else(|| "KDE layout not found: by or by:intl".to_string())?;
    let russian = find_layout_index(&layouts, "ru", None)
        .ok_or_else(|| "KDE layout not found: ru".to_string())?;

    eprintln!(
        "caps-layout-switcher: KDE layout indexes: us={english}, by={belarusian}, ru={russian}"
    );

    Ok(LayoutTargets {
        english,
        belarusian,
        russian,
    })
}

fn apply_gesture(bus: UserBus, targets: LayoutTargets, gesture: CapsGesture) -> Result<(), String> {
    let index = match gesture {
        CapsGesture::SingleTap => targets.english,
        CapsGesture::DoubleTap => targets.belarusian,
        CapsGesture::Hold => targets.russian,
    };

    eprintln!("caps-layout-switcher: {gesture:?} -> layout index {index}");
    set_layout(bus, index)
}

fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
}
