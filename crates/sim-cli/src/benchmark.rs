use serde::{Deserialize, Serialize};
use serde_json::json;
use sim_core::{
    Action, CommandNumber, Floor, FloorId, Geometry, InitialOccupancy, InitialPerson, LiftId,
    LiftStop, LiftStopId, Location, OperatingRules, Outcome, PersonId, Place, PlaceId, PlaceKind,
    Scenario, ScheduledCommand, Section, SectionId, Stair, StairId, Walkway, WalkwayId, World,
};
use std::{
    collections::BTreeSet,
    error::Error,
    ffi::OsString,
    fs,
    num::NonZeroU64,
    path::Path,
    thread,
    time::{Duration, Instant},
};

const TICKS_PER_SECOND: u64 = 10;
const TICK_MILLIS: u64 = 1000 / TICKS_PER_SECOND;
const SETUP_TICK: u64 = 3;
const LIFT_COST: u32 = 35;
const STAIR_TICKS: NonZeroU64 = NonZeroU64::new(60).unwrap();
const FLOOR_WIDTH: u32 = 8;
const STAIR_COLUMN: u32 = 1;
const OFFICE_COLUMN: u32 = 2;
const HOME_COLUMN: u32 = 7;
const USAGE: &str =
    "Aufruf: sim-cli bench KONFIG (--seconds N | --ticks N) --out DATEI [--scenario-out DATEI]";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u32,
    seed: u64,
    floors: u32,
    people: u32,
    cross_floor_per_floor: u32,
    home_floor_offset: u32,
    wave_tick: NonZeroU64,
    quiet_from_tick: u64,
    observed_office_floor: u32,
    feeders: Vec<Line>,
    express: Line,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Line {
    first_floor: u32,
    last_floor: u32,
    column: u32,
}

struct Options<'a> {
    config: &'a Path,
    output: &'a Path,
    scenario_output: Option<&'a Path>,
    ticks: u64,
    paced: bool,
}

#[derive(Serialize)]
struct Transfer {
    person: PersonId,
    tick: u64,
    from_lift: LiftId,
    to_lift: LiftId,
    from_kind: &'static str,
    to_kind: &'static str,
}

struct Phase {
    name: &'static str,
    tick_times: Vec<Duration>,
    route_searches: u64,
    longest_queue: usize,
    peak_rss: Option<u64>,
}

pub(crate) fn run(args: &[OsString]) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let options = parse_options(args)?;
    let config: Config = serde_json::from_slice(&crate::read(options.config.as_os_str())?)
        .map_err(|error| format!("{}: {error}", options.config.display()))?;
    config.validate()?;
    let scenario = config.generate()?;
    if let Some(path) = options.scenario_output {
        write_json(path, &scenario)?;
    }
    let report = measure(&config, scenario, options.ticks, options.paced)?;
    write_json(options.output, &report)
}

fn parse_options(args: &[OsString]) -> Result<Options<'_>, Box<dyn Error>> {
    let (input, flags) = args.split_first().ok_or(USAGE)?;
    let config = Path::new(input);
    let (mut mode, mut output, mut scenario_output) = (None, None, None);
    for pair in flags.chunks(2) {
        let [flag, value] = pair else {
            return Err(format!("Wert für {:?} fehlt. {USAGE}", pair[0]).into());
        };
        match flag.to_str() {
            Some("--seconds" | "--ticks") if mode.is_none() => {
                let value = value
                    .to_str()
                    .and_then(|text| text.parse::<NonZeroU64>().ok())
                    .ok_or("Die Laufdauer muss eine positive ganze Zahl sein.")?;
                let paced = flag == "--seconds";
                let ticks = value
                    .get()
                    .checked_mul(if paced { TICKS_PER_SECOND } else { 1 })
                    .ok_or("Die angegebene Laufdauer ist zu groß.")?;
                mode = Some((ticks, paced));
            }
            Some("--out") if output.is_none() => output = Some(Path::new(value)),
            Some("--scenario-out") if scenario_output.is_none() => {
                scenario_output = Some(Path::new(value));
            }
            _ => {
                return Err(format!("Unbekannte oder wiederholte Option {flag:?}. {USAGE}").into());
            }
        }
    }
    let (ticks, paced) = mode.ok_or(USAGE)?;
    let output = output.ok_or(USAGE)?;
    if output == config || scenario_output.is_some_and(|path| path == config || path == output) {
        return Err("Konfiguration, Haus und Bericht benötigen verschiedene Dateipfade.".into());
    }
    if paced && ticks.checked_mul(TICK_MILLIS).is_none() {
        return Err("Die angegebene Laufdauer ist zu groß.".into());
    }
    Ok(Options {
        config,
        output,
        scenario_output,
        ticks,
        paced,
    })
}

impl Config {
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.version != 1 {
            return Err(format!("Unbekannte Benchmark-Version {}.", self.version).into());
        }
        if !(2..=u32::MAX / FLOOR_WIDTH).contains(&self.floors)
            || !self.people.is_multiple_of(self.floors)
            || !(1..=self.people / self.floors).contains(&self.cross_floor_per_floor)
            || !(1..self.floors).contains(&self.home_floor_offset)
            || self.observed_office_floor >= self.floors
        {
            return Err(
                "Etagen, Personen und Pendlergruppe passen nicht zur Gebäudebelegung.".into(),
            );
        }
        if self.wave_tick.get() <= SETUP_TICK || self.quiet_from_tick <= self.wave_tick.get() {
            return Err("Die Stoßzeit muss nach dem Aufbau und vor der Ruhephase beginnen.".into());
        }
        for (line, step) in self.lines() {
            if line.first_floor >= line.last_floor
                || line.last_floor >= self.floors
                || !(OFFICE_COLUMN + 1..HOME_COLUMN).contains(&line.column)
                || !line.first_floor.is_multiple_of(step)
                || !line.last_floor.is_multiple_of(step)
                || (step == 1 && line.last_floor - line.first_floor >= 15)
            {
                return Err(format!(
                    "Ungültige Linie von Etage {} bis {} in Spalte {}.",
                    line.first_floor, line.last_floor, line.column
                )
                .into());
            }
        }
        Ok(())
    }

    fn lines(&self) -> impl Iterator<Item = (Line, u32)> + '_ {
        self.feeders
            .iter()
            .copied()
            .map(|line| (line, 1))
            .chain([(self.express, 10)])
    }

    fn generate(&self) -> Result<Scenario, Box<dyn Error>> {
        let sections: Vec<_> = (0..self.floors)
            .map(|floor| Section {
                id: section(floor),
                floor: FloorId(floor),
                start: 0,
                end: FLOOR_WIDTH,
                rent_cost: 0,
            })
            .collect();
        let rented_sections = sections.iter().map(|section| section.id).collect();
        let lift_landings: BTreeSet<_> = self
            .lines()
            .flat_map(|(line, step)| line.floors(step).map(move |floor| (floor, line.column)))
            .collect();
        let lift_stops = lift_landings
            .iter()
            .map(|&(floor, column)| LiftStop {
                id: stop(floor, column),
                floor: FloorId(floor),
                column,
            })
            .collect();
        let mut places = vec![make_place(0, 0, PlaceKind::Reception)];
        let mut walkways = vec![walkway(1, place(0, 0), place(0, STAIR_COLUMN))];
        for floor in 0..self.floors {
            places.push(make_place(floor, STAIR_COLUMN, PlaceKind::Landing));
            let local_places = [
                (OFFICE_COLUMN, PlaceKind::Office),
                (HOME_COLUMN, PlaceKind::Home),
            ]
            .into_iter()
            .chain(
                lift_landings
                    .range((floor, 0)..=(floor, u32::MAX))
                    .map(|&(_, column)| (column, PlaceKind::Landing)),
            );
            for (column, kind) in local_places {
                places.push(make_place(floor, column, kind));
                walkways.push(walkway(
                    u32::try_from(walkways.len() + 1)?,
                    place(floor, STAIR_COLUMN),
                    place(floor, column),
                ));
            }
        }
        let stairs = (1..self.floors)
            .map(|floor| Stair {
                id: StairId(floor),
                from: place(floor - 1, STAIR_COLUMN),
                to: place(floor, STAIR_COLUMN),
                ticks: STAIR_TICKS,
            })
            .collect();
        let people_per_floor = self.people / self.floors;
        // Workplaces and homes in this workload represent shared destinations on each floor.
        let people = (0..self.people)
            .map(|index| {
                let floor = index / people_per_floor;
                let home_floor = if index % people_per_floor < self.cross_floor_per_floor {
                    (floor + self.home_floor_offset) % self.floors
                } else {
                    floor
                };
                let office = place(floor, OFFICE_COLUMN);
                InitialPerson {
                    id: PersonId(index + 1),
                    location: office,
                    home: place(home_floor, HOME_COLUMN),
                    office,
                    household: None,
                }
            })
            .collect();
        let geometry = Geometry {
            floors: (0..self.floors)
                .map(|floor| Floor {
                    id: FloorId(floor),
                    width: FLOOR_WIDTH,
                })
                .collect(),
            sections,
            places,
            walkways,
            stairs,
            lift_stops,
        };
        let initial = InitialOccupancy {
            balance: 1_000 + (i64::try_from(self.feeders.len())? + 1) * i64::from(LIFT_COST),
            rented_sections,
            people,
            applicants: Vec::new(),
        };
        let rules = OperatingRules {
            day_ticks: self.wave_tick,
            home_income: 0,
            office_income: 0,
            designation_cost: 4,
            lift_cost: LIFT_COST,
            population_points: 1,
            coffee_threshold: 2.try_into()?,
            wait_sample_ticks: NonZeroU64::new(10).unwrap(),
            route_switch_savings: 6,
            initiative_ticks: 5.try_into()?,
        };
        let house =
            json!({ "version": 1, "geometry": geometry, "initial": initial, "rules": rules });
        Ok(Scenario::from_json(&serde_json::to_vec(&house)?)?)
    }
}

impl Line {
    fn floors(self, step: u32) -> impl Iterator<Item = u32> {
        (self.first_floor..=self.last_floor).step_by(step as usize)
    }

    fn stops(self, step: u32) -> Vec<LiftStopId> {
        self.floors(step)
            .map(|floor| stop(floor, self.column))
            .collect()
    }
}

fn place(floor: u32, column: u32) -> PlaceId {
    PlaceId(floor * FLOOR_WIDTH + column + 1)
}

fn stop(floor: u32, column: u32) -> LiftStopId {
    LiftStopId(place(floor, column).0)
}

fn section(floor: u32) -> SectionId {
    SectionId(floor + 1)
}

fn make_place(floor: u32, column: u32, kind: PlaceKind) -> Place {
    Place {
        id: place(floor, column),
        section: section(floor),
        column,
        kind,
    }
}

fn walkway(id: u32, from: PlaceId, to: PlaceId) -> Walkway {
    Walkway {
        id: WalkwayId(id),
        from,
        to,
        ticks: NonZeroU64::MIN,
        bidirectional: true,
    }
}

impl Phase {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            tick_times: Vec::new(),
            route_searches: 0,
            longest_queue: 0,
            peak_rss: None,
        }
    }

    fn summarize(mut self) -> serde_json::Value {
        self.tick_times.sort_unstable();
        let ticks = self.tick_times.len();
        let rank = ticks.saturating_mul(95).div_ceil(100);
        json!({
            "phase": self.name,
            "ticks": ticks,
            "tick_p95_ms": rank.checked_sub(1).map(|index| millis(self.tick_times[index])),
            "tick_max_ms": self.tick_times.last().copied().map(millis),
            "route_searches": self.route_searches,
            "longest_queue": self.longest_queue,
            "peak_sampled_rss_bytes": self.peak_rss,
        })
    }
}

fn measure(
    config: &Config,
    scenario: Scenario,
    ticks: u64,
    paced: bool,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let mut world = World::new(scenario, config.seed)?;
    world.submit(ScheduledCommand {
        tick: 2,
        number: CommandNumber(1),
        action: Action::UnlockLift,
    })?;
    for (index, (line, step)) in config.lines().enumerate() {
        let stops = line.stops(step);
        world.submit(ScheduledCommand {
            tick: SETUP_TICK,
            number: CommandNumber(u64::try_from(index)? + 2),
            action: if step == 1 {
                Action::BuildLift { stops }
            } else {
                Action::BuildExpress { stops }
            },
        })?;
    }
    let express_id = LiftId(u32::try_from(config.feeders.len())? + 1);
    let first_index = config.observed_office_floor * (config.people / config.floors);
    let mut cohort: Vec<_> = (first_index..first_index + config.cross_floor_per_floor)
        .map(|index| (PersonId(index + 1), None))
        .collect();
    let mut transfer = None;
    let mut phases = [
        Phase::new("Anlauf"),
        Phase::new("Stoßzeit"),
        Phase::new("Ruhephase"),
    ];
    let mut previous_searches = 0;
    let mut process_peak_rss = None;
    let start = Instant::now();
    if paced
        && start
            .checked_add(Duration::from_millis(ticks * TICK_MILLIS))
            .is_none()
    {
        return Err("Die angegebene Laufdauer ist zu groß.".into());
    }
    for tick in 1..=ticks {
        if paced {
            let deadline = start + Duration::from_millis(tick * TICK_MILLIS);
            thread::sleep(deadline.saturating_duration_since(Instant::now()));
        }
        let compute_start = Instant::now();
        let report = world.step();
        let compute_time = compute_start.elapsed();
        for result in report.results {
            if let Outcome::Rejected { reason } = result.outcome {
                return Err(format!(
                    "Aufbaubefehl {} bei Tick {tick} fehlgeschlagen. {reason}",
                    result.number.0
                )
                .into());
            }
        }
        let phase = if tick < config.wave_tick.get() {
            &mut phases[0]
        } else if tick < config.quiet_from_tick {
            &mut phases[1]
        } else {
            &mut phases[2]
        };
        if paced && phase.tick_times.is_empty() {
            eprintln!("Berufsverkehr. {} beginnt bei Tick {tick}.", phase.name);
        }
        phase.tick_times.push(compute_time);
        let stats = world.transport_stats();
        phase.route_searches += stats.route_searches - previous_searches;
        previous_searches = stats.route_searches;
        phase.longest_queue = phase.longest_queue.max(stats.waiting_people);
        let (resident, process_peak) = resident_memory();
        phase.peak_rss = phase.peak_rss.max(resident);
        process_peak_rss = process_peak_rss.max(process_peak);
        if transfer.is_none() {
            transfer = observe_transfer(&world, &mut cohort, express_id);
        }
    }
    let elapsed = start.elapsed();
    let stats = world.transport_stats();
    let checkpoint = world.checkpoint();
    Ok(json!({
        "version": 1,
        "configuration": config,
        "clock": if paced { "real_time" } else { "fixed_ticks" },
        "ticks_per_second": TICKS_PER_SECOND,
        "elapsed_ms": millis(elapsed),
        "peak_process_rss_bytes": process_peak_rss,
        "deterministic": {
            "ticks": checkpoint.report.tick,
            "checksum": checkpoint.checksum,
            "route_searches": stats.route_searches,
            "lift_boardings": stats.lift_boardings,
            "lift_transfers": stats.lift_transfers,
            "total_waited_ticks": stats.total_waited_ticks,
            "longest_queue": stats.longest_queue,
            "waiting_at_end": stats.waiting_people,
            "cohort_transfer": transfer,
        },
        "phases": phases.map(Phase::summarize),
    }))
}

fn observe_transfer(
    world: &World,
    cohort: &mut [(PersonId, Option<LiftId>)],
    express: LiftId,
) -> Option<Transfer> {
    let kind = |lift| if lift == express { "express" } else { "feeder" };
    for (person, last_lift) in cohort {
        let location = world.person_location(*person);
        if let Some(Location::At { .. }) = location {
            *last_lift = None;
        }
        if let Some(Location::RidingLift { lift, .. }) = location
            && let Some(previous) = last_lift.replace(lift)
            && kind(previous) != kind(lift)
        {
            return Some(Transfer {
                person: *person,
                tick: world.tick(),
                from_lift: previous,
                to_lift: lift,
                from_kind: kind(previous),
                to_kind: kind(lift),
            });
        }
    }
    None
}

fn resident_memory() -> (Option<u64>, Option<u64>) {
    #[cfg(target_os = "linux")]
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        let bytes = |key| {
            status.lines().find_map(|line| {
                line.strip_prefix(key)?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()?
                    .checked_mul(1024)
            })
        };
        return (bytes("VmRSS:"), bytes("VmHWM:"));
    }
    (None, None)
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn write_json(path: &Path, contents: &impl Serialize) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(contents)?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()).into())
}
