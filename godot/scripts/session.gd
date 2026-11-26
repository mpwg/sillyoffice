extends Control

const Replay := preload("res://tests/replay.gd")
const Selection := preload("res://tests/selection.gd")
const DesktopSmoke := preload("res://tests/desktop_smoke.gd")
const SaveStoreTest := preload("res://tests/save_store.gd")
const SaveStore := preload("res://scripts/save_store.gd")
const PROTOTYPE := "res://content/erstes-haus.json"
const SAVE_PATH := "user://spielstand.json"
const SECONDS_PER_TICK := 0.1

@onready var _building := $Layout/Body/Building
@onready var _sidebar: ScrollContainer = $Layout/Body/Sidebar
@onready var _person_panel := $Layout/Body/Sidebar/Content/PersonPanel
@onready var _build_menu := $Layout/Body/Sidebar/Content/BuildMenu
@onready var _status: Label = $Layout/Status
@onready var _notice: Label = $Layout/Footer/Notice
@onready var _pause_button: Button = $Layout/Header/Pause
@onready var _step_button: Button = $Layout/Header/Step
@onready var _floor_picker: OptionButton = $Layout/Header/Floor

var _session: Object
var _store = SaveStore.new()
var _scenario_json := ""
var _is_prototype := false
var _full_frame: Dictionary = {}
var _frame: Dictionary = {}
var _floors: Array[int] = []
var _first_floor := 0
var _last_floor := 1
var _elapsed := 0.0
var _paused := false
var _selected_person_id := -1
var _selected_record: Dictionary = {}
var _following := false
var _queued_return: Dictionary = {}


func _ready() -> void:
	_building.person_pressed.connect(_select_person)
	_person_panel.person_requested.connect(_select_person)
	_person_panel.follow_changed.connect(_set_following)
	_build_menu.action_requested.connect(_handle_action)
	_floor_picker.item_selected.connect(_choose_floor)
	_pause_button.pressed.connect(func() -> void: set_paused(not _paused))
	_step_button.pressed.connect(_single_step)
	$Layout/Footer/Save.pressed.connect(_save)
	$Layout/Footer/Load.pressed.connect(_load)

	var args := OS.get_cmdline_user_args()
	if not args.is_empty() and args[0] == "--replay":
		get_tree().quit(await Replay.run(self, args))
		return
	if args == PackedStringArray(["--test", "selection"]):
		get_tree().quit(await Selection.run(self))
		return
	if args == PackedStringArray(["--test", "save-store"]):
		get_tree().quit(SaveStoreTest.run())
		return
	if args.size() >= 2 and args[0] == "--test" and args[1] == "desktop-smoke":
		get_tree().quit(DesktopSmoke.run(self, args.slice(2)))
		return

	var load_error := _start_prototype()
	if not load_error.is_empty():
		_show_error(load_error)
		return
	set_paused(true)
	_notice.text = "Widme eine Wohnung und ein erreichbares Büro für den ersten Einzug."


func _start_prototype() -> String:
	if not FileAccess.file_exists(PROTOTYPE):
		return "Partiedatei fehlt (%s)." % PROTOTYPE
	var candidate := _new_session()
	if candidate == null:
		return "Die Simulationsbibliothek konnte nicht geladen werden."
	var scenario_json := FileAccess.get_file_as_string(PROTOTYPE)
	var load_error: String = candidate.start_prototype(scenario_json, 42)
	if load_error.is_empty():
		_adopt_session(candidate, scenario_json, true)
	return load_error


func start_partie(scenario_path: String, replay_path: String) -> String:
	for path in [scenario_path, replay_path]:
		if not FileAccess.file_exists(path):
			return "Partiedatei fehlt (%s)." % path
	var candidate := _new_session()
	if candidate == null:
		return "Die Simulationsbibliothek konnte nicht geladen werden."

	var scenario_json := FileAccess.get_file_as_string(scenario_path)
	var load_error: String = candidate.load_partie(
		scenario_json, FileAccess.get_file_as_string(replay_path)
	)
	if load_error.is_empty():
		_adopt_session(candidate, scenario_json, false)
	return load_error


func _new_session() -> Object:
	if not ClassDB.class_exists(&"SimSession") or not ClassDB.can_instantiate(&"SimSession"):
		return null
	return ClassDB.instantiate(&"SimSession")


func _adopt_session(candidate: Object, scenario_json: String, is_prototype: bool) -> void:
	_session = candidate
	_scenario_json = scenario_json
	_is_prototype = is_prototype
	_elapsed = 0.0
	_queued_return.clear()
	_build_menu.clear_preview()
	_configure_floors()
	_refresh_snapshot()
	_building.fit_columns(_full_frame.get("places", []), _full_frame.get("sections", []))


func _configure_floors() -> void:
	var scenario := _dictionary(_scenario_json)
	_floors.clear()
	for floor_geometry in scenario.geometry.floors:
		_floors.append(int(floor_geometry.id))
	_floors.sort()
	_floor_picker.clear()
	_floor_picker.add_item("Übersicht")
	for floor_number in _floors:
		var title := "Erdgeschoss" if floor_number == 0 else "%d. Etage" % floor_number
		_floor_picker.add_item(title)
	_first_floor = _floors.front()
	_last_floor = _floors.back()
	_floor_picker.select(0)


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


func _single_step() -> void:
	if _session == null:
		return
	set_paused(true)
	_elapsed = 0.0
	if not advance_tick():
		_show_error("Die Simulation konnte nicht weiterlaufen.")


func advance_tick() -> bool:
	if _session == null or not _session.step():
		return false
	_refresh_snapshot()
	if _is_prototype:
		_report_results()
	return true


func checkpoint_json() -> String:
	return _session.checkpoint_json()


func set_paused(paused: bool) -> void:
	_paused = paused
	_pause_button.text = "Fortsetzen" if paused else "Pause"
	_update_status()


func render_tick() -> float:
	return float(_full_frame.get("tick", 0)) + _elapsed / SECONDS_PER_TICK


func look_at_building() -> void:
	_first_floor = _floors.front()
	_last_floor = _floors.back()
	_floor_picker.select(0)
	_refresh_frame()


func look_at_floor(floor_number: int) -> Dictionary:
	_first_floor = floor_number
	_last_floor = floor_number
	_floor_picker.select(_floors.find(floor_number) + 1)
	_refresh_frame()
	return _frame


func _choose_floor(choice: int) -> void:
	_set_following(false)
	if choice == 0:
		look_at_building()
	else:
		look_at_floor(_floors[choice - 1])


func selected_person_id() -> int:
	return _selected_person_id


func _select_person(person_id: int) -> void:
	_selected_person_id = person_id
	_building.mark_selected(person_id)
	_refresh_selection()
	_follow_person()


func _set_following(enabled: bool) -> void:
	_following = enabled
	_person_panel.set_following(enabled)
	_follow_person()


func _follow_person() -> void:
	if not _following or not _selected_record.has("location"):
		return
	var location: Dictionary = _selected_record.location
	var floor_number: int
	if location.kind == "walking":
		floor_number = int(location.from.floor)
		if floor_number != int(location.to.floor):
			_follow_in_overview()
			return
	else:
		floor_number = int(location.place.floor)
		if location.kind == "riding_lift":
			for lift in _full_frame.get("lifts", []):
				if lift.id == location.lift and lift.get("motion") != null:
					_follow_in_overview()
					return
	if _first_floor != floor_number or _last_floor != floor_number:
		look_at_floor(floor_number)


func _follow_in_overview() -> void:
	if _first_floor != _floors.front() or _last_floor != _floors.back():
		look_at_building()


func _refresh_snapshot() -> void:
	_full_frame = _dictionary(_session.view_json(_floors.front(), _floors.back()))
	_update_status()
	_refresh_selection()
	_follow_person()
	_refresh_frame()


func _refresh_selection() -> void:
	_selected_record.clear()
	if _selected_person_id >= 0 and _is_prototype:
		_selected_record = _dictionary(_session.inspect_json(_selected_person_id))
	_person_panel.present(_selected_person_id, _selected_record)
	var active_id := _selected_person_id if _selected_record.has("id") else -1
	_person_panel.refresh_people(_full_frame.get("people", []))
	_build_menu.refresh(_full_frame, active_id)
	if active_id < 0 and _following:
		_set_following(false)


func _refresh_frame() -> void:
	if _first_floor == _floors.front() and _last_floor == _floors.back():
		_frame = _full_frame
	else:
		_frame = _dictionary(_session.view_json(_first_floor, _last_floor))
	_present_frame()


func _update_status() -> void:
	_status.text = "Tick %d%s  ·  Kasse %d  ·  Wartende Haushalte %d  ·  Entwicklungspunkte %d" % [
		_full_frame.get("tick", 0),
		" · Pause" if _paused else "",
		_full_frame.get("balance", 0),
		_full_frame.get("waiting_households", 0),
		_full_frame.get("development_points", 0),
	]


func _present_frame() -> void:
	_building.present(_frame, _first_floor, _last_floor, render_tick())


func _handle_action(action: Dictionary) -> void:
	if action.is_empty() or not _is_prototype:
		return
	var is_return: bool = action.kind == "return_section"
	if is_return and not _queued_return.is_empty():
		_notice.text = "Eine Rückgabe wartet bereits auf den nächsten Tick."
		return

	var serialized := JSON.stringify(action)
	var cost_text := ""
	if action.get("confirmation") == null:
		var preview := _dictionary(_session.preview_json(serialized))
		if is_return:
			_build_menu.show_preview(action, preview)
			return
		if not preview.get("ok", false):
			_notice.text = preview.get("error", "Die Vorschau konnte nicht erstellt werden.")
			return
		var cost: int = int(preview.impact.get("immediate_cost", 0))
		if cost > 0:
			cost_text = " · Kosten %d" % cost
	var submit_error: String = _session.submit_action(serialized)
	if not submit_error.is_empty():
		_notice.text = submit_error
		return
	if is_return:
		_queued_return = {"kind": "return_section", "section": action.section}
	_notice.text = "Auftrag für den nächsten Tick angenommen" + cost_text


func _report_results() -> void:
	var results: Variant = JSON.parse_string(_session.results_json())
	if results is Dictionary:
		results = results.get("results", [])
	if results is not Array or results.is_empty():
		return

	var notices := PackedStringArray()
	var return_action := _queued_return.duplicate()
	_queued_return.clear()
	for result in results:
		var outcome: Dictionary = result.outcome
		if outcome.kind == "applied":
			notices.append("Auftrag %d ausgeführt" % result.number)
			continue
		var cause: Variant = outcome.get("reason", "")
		var code: String = str(cause.get("kind", "")) if cause is Dictionary else str(cause)
		notices.append("Auftrag %d abgelehnt · %s" % [result.number, _rule_reason(code)])
		if code == "return_changed" and not return_action.is_empty():
			_handle_action(return_action)
	_notice.text = "  ·  ".join(notices)


func _rule_reason(code: String) -> String:
	match code:
		"already_rented":
			return "Abschnitt ist bereits gemietet"
		"insufficient_funds":
			return "Kasse reicht nicht aus"
		"return_changed":
			return "Folgen haben sich geändert"
		_:
			return code.replace("_", " ").capitalize() if not code.is_empty() else "Regel verletzt"


func _save() -> void:
	if not _is_prototype:
		return
	var save_error := save_game(SAVE_PATH)
	_notice.text = "Partie gespeichert." if save_error.is_empty() else save_error


func save_game(path: String) -> String:
	if _session == null:
		return "Es ist keine Partie geladen."
	var payload: String = _session.save_json()
	var validation := func(contents: String) -> bool:
		return _session.validate_save(_scenario_json, contents)
	return _store.save(path, payload, validation)


func _load() -> void:
	if not _is_prototype:
		return
	var load_error := load_game(SAVE_PATH, PROTOTYPE)
	_notice.text = "Partie geladen." if load_error.is_empty() else load_error


func load_game(path: String, scenario_path: String) -> String:
	if not FileAccess.file_exists(scenario_path):
		return "Partiedatei fehlt (%s)." % scenario_path
	var scenario_json := FileAccess.get_file_as_string(scenario_path)
	var load_error := "Kein lesbarer Spielstand gefunden."
	for contents in _store.load_candidates(path):
		var candidate := _new_session()
		if candidate == null:
			return "Die Simulationsbibliothek konnte nicht geladen werden."
		load_error = candidate.restore_json(scenario_json, contents)
		if load_error.is_empty():
			_adopt_session(candidate, scenario_json, true)
			set_paused(true)
			return ""
	return load_error


func _dictionary(serialized: String) -> Dictionary:
	var result: Variant = JSON.parse_string(serialized)
	return result if result is Dictionary else {}


func _show_error(message: String) -> void:
	_notice.text = message
	_sidebar.hide()
	_floor_picker.disabled = true
	_pause_button.disabled = true
	_step_button.disabled = true
	$Layout/Footer/Save.disabled = true
	$Layout/Footer/Load.disabled = true
	set_process(false)
	push_error(message)
