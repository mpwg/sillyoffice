extends Control

const FRAME_COLOR := Color("273a43")
const FLOOR_COLOR := Color("3b5258")
const LIGHT_COLOR := Color("f4ddae")
const PERSON_COLOR := Color("ff9858")
const HORIZONTAL_MARGIN := 24.0

var _frame: Dictionary = {}
var _first_floor := 0
var _last_floor := 1
var _first_column := 0
var _last_column := 0
var _sample_tick := 0.0


func fit_columns(places: Array, sections: Array) -> void:
	_first_column = 0
	_last_column = 0
	for place in places:
		_first_column = mini(_first_column, int(place.column))
		_last_column = maxi(_last_column, int(place.column))
	for section in sections:
		_first_column = mini(_first_column, int(section.start))
		_last_column = maxi(_last_column, int(section.end))
	queue_redraw()


func present(frame: Dictionary, first_floor: int, last_floor: int, sample_tick: float) -> void:
	_frame = frame
	_first_floor = first_floor
	_last_floor = last_floor
	_sample_tick = sample_tick
	queue_redraw()


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, size), FRAME_COLOR)
	if _frame.is_empty():
		return
	for floor_number in range(_first_floor, _last_floor + 1):
		var height := _point(0.0, float(floor_number)).y
		draw_rect(Rect2(HORIZONTAL_MARGIN, height - 70.0, size.x - 2.0 * HORIZONTAL_MARGIN, 75.0), FLOOR_COLOR)
		draw_line(Vector2(HORIZONTAL_MARGIN, height + 5.0), Vector2(size.x - HORIZONTAL_MARGIN, height + 5.0), LIGHT_COLOR, 2.0)
	for section in _frame.sections:
		_draw_section(section)
	for place in _frame.places:
		_draw_place(place)
	for person in _frame.people:
		_draw_person(person)


func _draw_section(section: Dictionary) -> void:
	var start := _point(float(section.start), float(section.floor))
	var end := _point(float(section.end), float(section.floor))
	var color := Color("547d70") if section.rented else Color("556068")
	draw_rect(Rect2(start.x - 15.0, start.y - 62.0, end.x - start.x + 30.0, 23.0), color)
	var title := "Gemietet" if section.rented else "Frei"
	_draw_label(title, (start + end) * 0.5 + Vector2(0.0, -46.0))


func _draw_place(place: Dictionary) -> void:
	var point := _place_point(place)
	var title: String
	match place.kind:
		"reception":
			title = "Rezeption"
		"office":
			title = "Büro"
		"landing":
			title = "Treppe"
		"home":
			title = "Wohnung"
		_:
			title = String(place.kind).capitalize()
	draw_rect(Rect2(point + Vector2(-13.0, -31.0), Vector2(26.0, 31.0)), Color("876450"))
	draw_rect(Rect2(point + Vector2(-7.0, -25.0), Vector2(14.0, 25.0)), Color("b98a63"))
	_draw_label(title, point + Vector2(0.0, 24.0))


func _draw_person(person: Dictionary) -> void:
	var location: Dictionary = person.location
	var point: Vector2
	if location.kind == "walking":
		var start_tick := float(location.start_tick)
		var end_tick := float(location.end_tick)
		var fraction := clampf(inverse_lerp(start_tick, end_tick, _sample_tick), 0.0, 1.0)
		point = _place_point(location.from).lerp(_place_point(location.to), fraction)
	else:
		point = _place_point(location.place)
	draw_line(point + Vector2(0.0, -14.0), point, PERSON_COLOR, 8.0, true)
	draw_circle(point + Vector2(0.0, -19.0), 6.0, PERSON_COLOR, true, -1.0, true)


func _place_point(place: Dictionary) -> Vector2:
	return _point(float(place.column), float(place.floor))


func _point(column: float, floor_number: float) -> Vector2:
	var column_step := (size.x - 2.0 * HORIZONTAL_MARGIN) / float(_last_column - _first_column + 2)
	var floor_step := size.y / float(_last_floor - _first_floor + 1)
	return Vector2(
		HORIZONTAL_MARGIN + (column - _first_column + 1.0) * column_step,
		(_last_floor - floor_number + 0.56) * floor_step
	)


func _draw_label(title: String, center_baseline: Vector2) -> void:
	var font := ThemeDB.fallback_font
	var width := font.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x
	draw_string(font, center_baseline - Vector2(width * 0.5, 0.0), title, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, LIGHT_COLOR)
