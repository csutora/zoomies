use clap::Parser;
use evdev::{AbsoluteAxisType, InputEventKind, Key};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_pointer,
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

#[derive(Parser, Clone)]
#[command(about = "momentum scrolling for touchpads on wlroots-based compositors")]
struct Args {
    #[arg(long, default_value = "Magic Trackpad")]
    device_name: String,

    #[arg(long, default_value_t = 0.10)]
    multiplier: f64,

    #[arg(long, default_value_t = 325.0)]
    decay_ms: f64,

    #[arg(long, default_value_t = 200.0)]
    min_velocity: f64,

    #[arg(long, default_value_t = 40.0)]
    stop_threshold: f64,

    #[arg(long, default_value_t = 8)]
    tick_ms: u64,

    #[arg(long)]
    traditional: bool,
}

enum Msg {
    Start { velocity: f64, horizontal: bool },
    Stop,
}

const RING: usize = 8;
const SAMPLES: usize = 4;
const STALE_US: u64 = 150_000;

struct Ring {
    buf: [(f64, u64); RING],
    pos: usize,
    count: usize,
}

impl Ring {
    fn new() -> Self {
        Self {
            buf: [(0.0, 0); RING],
            pos: 0,
            count: 0,
        }
    }
    fn push(&mut self, delta: f64, ts_us: u64) {
        self.buf[self.pos] = (delta, ts_us);
        self.pos = (self.pos + 1) % RING;
        if self.count < RING {
            self.count += 1;
        }
    }
    fn clear(&mut self) {
        self.count = 0;
        self.pos = 0;
    }
    fn velocity(&self, now_us: u64) -> f64 {
        let n = self.count.min(SAMPLES);
        if n < 2 {
            return 0.0;
        }
        let newest = if self.pos == 0 {
            RING - 1
        } else {
            self.pos - 1
        };
        if now_us.saturating_sub(self.buf[newest].1) > STALE_US {
            return 0.0;
        }
        let start = if self.pos >= n {
            self.pos - n
        } else {
            RING - (n - self.pos)
        };
        let mut total = 0.0;
        let first_ts = self.buf[start].1;
        let mut last_ts = first_ts;
        for i in 1..n {
            let idx = (start + i) % RING;
            total += self.buf[idx].0;
            last_ts = self.buf[idx].1;
        }
        let dt = last_ts.saturating_sub(first_ts);
        if dt == 0 {
            return 0.0;
        }
        total / (dt as f64 / 1_000_000.0)
    }
}

fn ts_us(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

fn find_device(name: &str) -> Option<evdev::Device> {
    evdev::enumerate()
        .map(|(_, d)| d)
        .find(|d| d.name().is_some_and(|n| n.contains(name)))
}

fn run_listener(mut dev: evdev::Device, tx: &mpsc::Sender<Msg>, args: &Args) {
    let dir = if args.traditional { 1.0 } else { -1.0 };
    let mut scrolling = false;
    let mut ring_x = Ring::new();
    let mut ring_y = Ring::new();
    let mut prev_x = 0i32;
    let mut prev_y = 0i32;
    let mut just_started;

    while let Ok(events) = dev.fetch_events() {
        just_started = false;
        for ev in events {
            let now = ts_us(ev.timestamp());
            match ev.kind() {
                InputEventKind::AbsAxis(AbsoluteAxisType::ABS_X) => {
                    let v = ev.value();
                    if scrolling {
                        ring_x.push((v - prev_x) as f64, now);
                    }
                    prev_x = v;
                }
                InputEventKind::AbsAxis(AbsoluteAxisType::ABS_Y) => {
                    let v = ev.value();
                    if scrolling {
                        ring_y.push((v - prev_y) as f64, now);
                    }
                    prev_y = v;
                }
                InputEventKind::Key(Key::BTN_TOOL_DOUBLETAP) => {
                    if ev.value() == 1 {
                        let _ = tx.send(Msg::Stop);
                        scrolling = true;
                        ring_x.clear();
                        ring_y.clear();
                    } else if scrolling {
                        scrolling = false;
                        let vx = ring_x.velocity(now) * args.multiplier;
                        let vy = ring_y.velocity(now) * args.multiplier;
                        let (velocity, horizontal) = if vy.abs() >= vx.abs() {
                            (dir * vy, false)
                        } else {
                            (dir * vx, true)
                        };
                        if velocity.abs() >= args.min_velocity {
                            let _ = tx.send(Msg::Start {
                                velocity,
                                horizontal,
                            });
                            just_started = true;
                        }
                    }
                }
                InputEventKind::Key(Key::BTN_TOOL_FINGER) if ev.value() == 1 && !just_started => {
                    let _ = tx.send(Msg::Stop);
                }
                InputEventKind::Key(Key::BTN_TOOL_TRIPLETAP)
                | InputEventKind::Key(Key::BTN_TOOL_QUADTAP)
                | InputEventKind::Key(Key::BTN_TOOL_QUINTTAP)
                    if ev.value() == 1 =>
                {
                    scrolling = false;
                    let _ = tx.send(Msg::Stop);
                }
                _ => {}
            }
        }
    }
}

struct WState;

impl Dispatch<WlRegistry, GlobalListContents> for WState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlSeat, ()> for WState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
delegate_noop!(WState: ZwlrVirtualPointerManagerV1);
delegate_noop!(WState: ZwlrVirtualPointerV1);

fn run_emitter(rx: mpsc::Receiver<Msg>, args: Args) -> anyhow::Result<()> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<WState>(&conn)?;
    let qh = queue.handle();
    let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
    let mgr: ZwlrVirtualPointerManagerV1 = globals.bind(&qh, 1..=2, ())?;
    let vp = mgr.create_virtual_pointer(Some(&seat), &qh, ());
    queue.roundtrip(&mut WState)?;

    let tick = Duration::from_millis(args.tick_ms);
    let start = Instant::now();

    let emit = |horizontal: bool, value: f64| {
        let axis = if horizontal {
            wl_pointer::Axis::HorizontalScroll
        } else {
            wl_pointer::Axis::VerticalScroll
        };
        // order matters: on hyprland the axis request resets the axis source to
        // its wheel default, so axis_source has to come after axis. otherwise the
        // scroll arrives as a wheel event and apps quantize it to whole lines
        // instead of treating it as smooth finger-source scrolling.
        vp.axis(start.elapsed().as_millis() as u32, axis, value);
        vp.axis_source(wl_pointer::AxisSource::Finger);
        vp.frame();
        let _ = conn.flush();
    };

    let mut velocity = 0.0f64;
    let mut horizontal = false;
    let mut coasting = false;
    let mut last = Instant::now();

    loop {
        let msg = if coasting {
            rx.recv_timeout(tick)
        } else {
            rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        };
        match msg {
            Ok(Msg::Start {
                velocity: v,
                horizontal: h,
            }) => {
                velocity = v;
                horizontal = h;
                coasting = true;
                last = Instant::now();
            }
            Ok(Msg::Stop) => {
                if coasting {
                    emit(horizontal, 0.0);
                    coasting = false;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        if coasting {
            let dt = last.elapsed().as_secs_f64();
            last = Instant::now();
            velocity *= (-dt * 1000.0 / args.decay_ms).exp();
            if velocity.abs() < args.stop_threshold {
                emit(horizontal, 0.0);
                coasting = false;
            } else {
                emit(horizontal, velocity * dt);
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (tx, rx) = mpsc::channel::<Msg>();

    let in_args = args.clone();
    thread::Builder::new()
        .name("listener".into())
        .spawn(move || {
            loop {
                match find_device(&in_args.device_name) {
                    Some(dev) => run_listener(dev, &tx, &in_args),
                    None => thread::sleep(Duration::from_secs(2)),
                }
            }
        })?;

    run_emitter(rx, args)
}
