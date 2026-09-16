pub struct Args {
    pub render: Option<String>,
    pub bench: bool,
    pub bench_aa: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub width: u32,
    pub height: u32,
    pub night: bool,
    pub seed: u32,
    pub no_normalmaps: bool,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            render: None,
            bench: false,
            bench_aa: false,
            yaw: 235.0,
            pitch: 25.0,
            dist: 110.0,
            width: 1280,
            height: 720,
            night: false,
            seed: 1337,
            no_normalmaps: false,
        }
    }
}

pub fn parse(raw: &[String]) -> Args {
    let mut args = Args::default();
    let mut i = 0;
    while i < raw.len() {
        let flag = raw[i].as_str();
        let mut next = || {
            i += 1;
            raw.get(i).cloned().unwrap_or_default()
        };
        match flag {
            "--render" => args.render = Some(next()),
            "--bench" => args.bench = true,
            "--bench-aa" => args.bench_aa = true,
            "--yaw" => args.yaw = next().parse().unwrap_or(args.yaw),
            "--pitch" => args.pitch = next().parse().unwrap_or(args.pitch),
            "--dist" => args.dist = next().parse().unwrap_or(args.dist),
            "--width" => args.width = next().parse().unwrap_or(args.width),
            "--height" => args.height = next().parse().unwrap_or(args.height),
            "--night" => args.night = true,
            "--seed" => args.seed = next().parse().unwrap_or(args.seed),
            "--no-normalmaps" => args.no_normalmaps = true,
            _ => {}
        }
        i += 1;
    }
    args
}
