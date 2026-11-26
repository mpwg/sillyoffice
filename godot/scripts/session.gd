extends Control

const Replay := preload("res://tests/replay.gd")
const SCENARIO := "res://content/zwei-etagen.json"
const REPLAY := "res://content/erster-weg.json"
const SECONDS_PER_TICK := 0.1
const RECEPTION_FLOOR := 0
const OFFICE_FLOOR := 1

@onready var _building := $Layout/Building
@onready var _status: Label = $Layout/Status
@onready var _pause_button: Button = $Layout/Controls/Pause

var _session: Object
var _frame: Dictionary = {}
var _first_floor := RECEPTION_FLOOR
var _last_floor := OFFICE_FLOOR
var _elapsed := 0.0
var _paused := false


func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	if not args.is_empty() and args[0] == "--replay":
		get_tree().quit(await Replay.run(self, args))
		return

	$Layout/Controls/Overview.pressed.connect(look_at_building)
	$Layout/Controls/Reception.pressed.connect(look_at_floor.bind(RECEPTION_FLOOR))
	$Layout/Controls/Office.pressed.connect(look_at_floor.bind(OFFICE_FLOOR))
	_pause_button.pressed.connect(func() -> void: set_paused(not _paused))

	var load_error := start_partie(SCENARIO, REPLAY)
	if not load_error.is_empty():
		_show_error(load_error)


func start_partie(scenario_path: String, replay_path: String) -> String:
	if not ClassDB.class_exists(&"SimSession") or not ClassDB.can_instantiate(&"SimSession"):
		return "Die Simulationsbibliothek konnte nicht geladen werden."
	for path in [scenario_path, replay_path]:
		if not FileAccess.file_exists(path):
			return "Partiedatei fehlt (%s)." % path

	var candidate: Object = ClassDB.instantiate(&"SimSession")
	var load_error: String = candidate.load_partie(
		FileAccess.get_file_as_string(scenario_path), FileAccess.get_file_as_string(replay_path)
	)
	if not load_error.is_empty():
		return load_error

	_session = candidate
	_elapsed = 0.0
	look_at_building()
	_building.fit_columns(_frame.get("places", []), _frame.get("sections", []))
	return ""


func _process(delta: float) -> void:
	if _session == null or _paused:
		return
	_elapsed += delta
	while _elapsed >= SECONDS_PER_TICK:
		_elapsed -= SECONDS_PER_TICK
		if not advance_tick():
			_show_error("Die Simulation konnte nicht weiterlaufen.")
			return
	_present_frame()


func advance_tick() -> bool:
	if not _session.step():
		return false
	_refresh_frame()
	return true


func checkpoint_json() -> String:
	return _session.checkpoint_json()


func set_paused(paused: bool) -> void:
	_paused = paused
	_pause_button.text = "Fortsetzen" if paused else "Pause"
	_update_status()


func render_tick() -> float:
	return float(_frame.get("tick", 0)) + _elapsed / SECONDS_PER_TICK


func look_at_building() -> void:
	_first_floor = RECEPTION_FLOOR
	_last_floor = OFFICE_FLOOR
	_refresh_frame()


func look_at_floor(floor_number: int) -> Dictionary:
	_first_floor = floor_number
	_last_floor = floor_number
	_refresh_frame()
	return _frame


func _refresh_frame() -> void:
	var frame: Variant = JSON.parse_string(_session.view_json(_first_floor, _last_floor))
	_frame = frame if frame is Dictionary else {}
	_update_status()
	_present_frame()


func _update_status() -> void:
	var completed_tick := int(_frame.get("tick", 0))
	_status.text = "Tick %d · Pause" % completed_tick if _paused else "Tick %d" % completed_tick


func _present_frame() -> void:
	_building.present(_frame, _first_floor, _last_floor, render_tick())


func _show_error(message: String) -> void:
	_status.text = message
	$Layout/Controls.hide()
	set_process(false)
	push_error(message)
