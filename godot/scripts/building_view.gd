extends Control

signal person_pressed(person_id: int)

const FRAME_COLOR := Color("282037")
const FLOOR_COLOR := Color("493547")
const LIGHT_COLOR := Color("f4ddae")
const PERSON_COLOR := Color("ff9858")
const HORIZONTAL_MARGIN := 34.0
const ROOM_NAMES := {
	"reception": "Rezeption", "office": "Büro", "landing": "Treppe",
	"home": "Wohnung", "kiosk": "Kiosk", "corridor": "Flur", "vacant": "Fläche",
}
const ROOM_COLORS := {
	"reception": Color("bb744c"), "office": Color("5c8191"), "home": Color("9a6356"),
	"kiosk": Color("b76b52"), "landing": Color("75687b"), "corridor": Color("75658a"),
}

var _frame: Dictionary = {}
var _first_floor := 0
var _last_floor := 1
var _first_column := 0
var _last_column := 0
var _sample_tick := 0.0
var _selected_person_id := -1


func fit_columns(places: Array, sections: Array) -> void:
	_first_column = 0
	_last_column = 0
	for place in places:
		_first_column = mini(_first_column, int(place.column))
		_last_column = maxi(_last_column, int(place.column))
	for section in sections:
		_first_column = mini(_first_column, int(section.start))
		_last_column = maxi(_last_column, int(section.end) - 1)
	queue_redraw()


func present(frame: Dictionary, first_floor: int, last_floor: int, sample_tick: float) -> void:
	_frame = frame
	_first_floor = first_floor
	_last_floor = last_floor
	_sample_tick = sample_tick
	queue_redraw()


func mark_selected(person_id: int) -> void:
	_selected_person_id = person_id
	queue_redraw()


func _gui_input(event: InputEvent) -> void:
	if event is not InputEventMouseButton or not event.pressed:
		return
	if event.button_index != MOUSE_BUTTON_LEFT:
		return

	var candidate := -1
	var nearest := 18.0
	for person in _frame.get("people", []):
		var center := _person_point(person) + Vector2(0.0, -15.0)
		if not Rect2(Vector2.ZERO, size).has_point(center):
			continue
		var distance: float = center.distance_to(event.position)
		if distance < nearest:
			candidate = int(person.id)
			nearest = distance
	if candidate >= 0:
		person_pressed.emit(candidate)
		accept_event()


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, size), FRAME_COLOR)
	if _frame.is_empty():
		return
	for floor_number in range(_first_floor, _last_floor + 1):
		_draw_floor(floor_number)
	for section in _frame.sections:
		_draw_section(section)
	for place in _frame.places:
		_draw_place(place)
	for stop in _frame.get("stops", []):
		_draw_stop(stop)
	for lift in _frame.get("lifts", []):
		_draw_lift(lift)
	for initiative in _frame.get("initiatives", []):
		_draw_initiative(initiative)
	for person in _frame.people:
		_draw_person(person)


func _draw_floor(floor_number: int) -> void:
	var height := _point(0.0, float(floor_number)).y
	var left := HORIZONTAL_MARGIN - 8.0
	var right := size.x - HORIZONTAL_MARGIN + 8.0
	draw_rect(Rect2(left, height - 66.0, right - left, 72.0), FLOOR_COLOR)
	draw_line(Vector2(left, height + 6.0), Vector2(right, height + 6.0), Color("a27161"), 4.0)
	var name_tag := "EG" if floor_number == 0 else "%d" % floor_number
	_draw_label(name_tag, Vector2(left * 0.5, height), 12)


func _draw_section(section: Dictionary) -> void:
	var start := _point(float(section.start) - 0.45, float(section.floor))
	var end := _point(float(section.end) - 0.55, float(section.floor))
	var color := Color("605855") if section.rented else Color("695265")
	draw_rect(Rect2(start.x, start.y - 64.0, end.x - start.x, 18.0), color)
	var tenancy := "gemietet" if section.rented else "frei"
	_draw_label(
		"%d · %s" % [section.id, tenancy], (start + end) * 0.5 + Vector2(0.0, -51.0), 11
	)


func _draw_place(place: Dictionary) -> void:
	var point := _place_point(place)
	var room_key: Variant = place.get("use")
	if room_key == null:
		room_key = place.kind
	var title: String = ROOM_NAMES.get(room_key, "Fläche")
	if place.kind == "space":
		title += " %d" % place.id
	var color: Color = ROOM_COLORS.get(room_key, Color("554350"))
	var width := clampf(_column_step() - 6.0, 14.0, 60.0)
	draw_rect(Rect2(point + Vector2(-width * 0.5, -41.0), Vector2(width, 41.0)), color)
	draw_line(
		point + Vector2(-width * 0.5, -41.0), point + Vector2(width * 0.5, -41.0),
		Color("cf9b72"), 2.0
	)
	draw_rect(Rect2(point + Vector2(-6.0, -24.0), Vector2(12.0, 24.0)), Color("423343"))
	if width >= 15.0:
		var label_offset := 20.0 + float(int(place.column) % 2) * 14.0
		_draw_label(title, point + Vector2(0.0, label_offset), 11)


func _draw_stop(stop: Dictionary) -> void:
	var point := _place_point(stop) + Vector2(13.0, 0.0)
	draw_rect(Rect2(point + Vector2(-14, -42), Vector2(28, 42)), Color("44415e"))
	draw_rect(Rect2(point + Vector2(-11, -35), Vector2(22, 35)), Color("676683"), false, 1.0)
	_draw_label("H%d" % stop.id, point + Vector2(0, -30), 10)


func _draw_lift(lift: Dictionary) -> void:
	var point := _lift_point(lift)
	draw_rect(Rect2(point + Vector2(-11, -29), Vector2(22, 29)), Color("9a8aa9"))
	draw_line(point + Vector2(0, -29), point, Color("493d60"), 2.0)
	var rider_count := int(lift.riders)
	var waiting_count := int(lift.waiting)
	if rider_count + waiting_count > 0:
		_draw_label("%d ↑ · %d warten" % [rider_count, waiting_count], point + Vector2(0, 26), 11)


func _draw_initiative(initiative: Dictionary) -> void:
	var from := _place_point(initiative.from) + Vector2(4, -6)
	var to := _place_point(initiative.to) + Vector2(4, -6)
	if initiative.built:
		draw_line(from, to, Color("de8773"), 5.0, true)
		draw_line(from, to, Color("ffb482"), 1.5, true)
	else:
		draw_dashed_line(from, to, Color("de8773"), 2.0, 6.0, true, true)


func _draw_person(person: Dictionary) -> void:
	var point := _person_point(person)
	if person.id == _selected_person_id:
		draw_circle(point + Vector2(0.0, -15.0), 13.0, LIGHT_COLOR, false, 2.0, true)
	var shirt := PERSON_COLOR if int(person.id) % 2 == 1 else Color("a5d8c6")
	draw_line(point + Vector2(0.0, -14.0), point, shirt, 8.0, true)
	draw_circle(point + Vector2(0.0, -19.0), 6.0, LIGHT_COLOR, true, -1.0, true)


func _person_point(person: Dictionary) -> Vector2:
	var location: Dictionary = person.location
	var point: Vector2
	if location.kind == "walking":
		point = _place_point(location.from).lerp(_place_point(location.to), _motion_progress(location))
	else:
		point = _place_point(location.place)
		if location.kind == "riding_lift":
			for lift in _frame.get("lifts", []):
				if lift.id == location.lift:
					point = _lift_point(lift)
					break
	return point + Vector2(float(int(person.id) % 3 - 1) * 5.0, 0.0)


func _lift_point(lift: Dictionary) -> Vector2:
	var floor_number := float(lift.floor)
	var motion: Variant = lift.get("motion")
	if motion is Dictionary:
		floor_number = lerpf(float(motion.from_floor), float(motion.to_floor), _motion_progress(motion))
	return _point(float(lift.column), floor_number) + Vector2(13.0, 0.0)


func _motion_progress(segment: Dictionary) -> float:
	return clampf(inverse_lerp(float(segment.start_tick), float(segment.end_tick), _sample_tick), 0.0, 1.0)


func _place_point(place: Dictionary) -> Vector2:
	return _point(float(place.column), float(place.floor))


func _column_step() -> float:
	return (size.x - 2.0 * HORIZONTAL_MARGIN) / float(_last_column - _first_column + 2)


func _point(column: float, floor_number: float) -> Vector2:
	var floor_step := size.y / float(_last_floor - _first_floor + 1)
	return Vector2(
		HORIZONTAL_MARGIN + (column - _first_column + 1.0) * _column_step(),
		(_last_floor - floor_number + 0.61) * floor_step
	)


func _draw_label(title: String, center_baseline: Vector2, font_size: int) -> void:
	var font := ThemeDB.fallback_font
	var width := font.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
	draw_string(
		font, center_baseline - Vector2(width * 0.5, 0.0), title,
		HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, LIGHT_COLOR
	)
