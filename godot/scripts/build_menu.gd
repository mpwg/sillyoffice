extends VBoxContainer

signal action_requested(action: Dictionary)

const ROOM_USES := {
	"home": "Wohnung",
	"office": "Büro",
	"kiosk": "Kiosk",
	"corridor": "Flur",
	"vacant": "Leerfläche",
}

var _section: OptionButton
var _room: OptionButton
var _room_use: OptionButton
var _first_stop: OptionButton
var _last_stop: OptionButton
var _kiosk: OptionButton
var _rent: Button
var _return_preview: Button
var _designate: Button
var _unlock: Button
var _build_lift: Button
var _coffee: Button
var _confirm: Button
var _preview: Label

var _choices := {}
var _stops: Array = []
var _selected_person := -1
var _pending_return: Dictionary = {}


func _ready() -> void:
	_heading("Miete & Raum")
	_section = _picker("Abschnitt wählen")
	_section.item_selected.connect(func(_index: int) -> void: clear_preview())
	var lease_actions := HBoxContainer.new()
	add_child(lease_actions)
	_rent = _button("Anmieten", _rent_section, lease_actions)
	_rent.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_return_preview = _button("Rückgabe prüfen", _preview_return, lease_actions)
	_return_preview.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_preview = Label.new()
	_preview.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_preview.add_theme_color_override("font_color", Color("f4ddae"))
	add_child(_preview)
	_confirm = _button("Rückgabe bestätigen", _confirm_return)
	clear_preview()

	_room = _picker("Fläche wählen")
	var uses := HBoxContainer.new()
	add_child(uses)
	_room_use = _picker("Nutzung wählen", uses)
	for room_use in ROOM_USES:
		_add_item(_room_use, ROOM_USES[room_use], room_use)
	_room_use.select(0)
	_designate = _button("Widmen", _designate_room, uses)

	_heading("Lift")
	_unlock = _button("Einfachen Lift freischalten", _unlock_lift)
	var range_row := HBoxContainer.new()
	add_child(range_row)
	_first_stop = _picker("Erster Halt", range_row)
	_last_stop = _picker("Letzter Halt", range_row)
	_build_lift = _button("Lift bauen", _request_lift)

	_heading("Kaffeegang")
	_kiosk = _picker("Kiosk wählen")
	_coffee = _button("Ausgewählte Person zum Kiosk schicken", _send_to_kiosk)

func refresh(frame: Dictionary, active_person_id: int) -> void:
	_selected_person = active_person_id
	var sections := []
	for section in frame.get("sections", []):
		var tenancy := "gemietet" if section.rented else "frei"
		sections.append(
			["Abschnitt %d · %s · %s" % [section.id, _floor_name(int(section.floor)), tenancy],
			 int(section.id)]
		)
	_set_choices(_section, sections)

	var rooms := []
	var kiosks := []
	for place in frame.get("places", []):
		var room_use: Variant = place.get("use")
		var usage: String = ROOM_USES.get(room_use, "Leerfläche")
		var title := "%s %d · %s" % [usage, place.id, _floor_name(int(place.floor))]
		if place.kind == "space":
			rooms.append([title, int(place.id)])
		if room_use == "kiosk":
			kiosks.append([title, int(place.id)])
	_set_choices(_room, rooms)
	_set_choices(_kiosk, kiosks)

	_stops = frame.get("stops", [])
	var stop_choices := []
	for stop in _stops:
		var title := "Halt %d · %s" % [stop.id, _floor_name(int(stop.floor))]
		stop_choices.append([title, int(stop.id)])
	_set_choices(_first_stop, stop_choices)
	if _set_choices(_last_stop, stop_choices) and not stop_choices.is_empty():
		_last_stop.select(stop_choices.size() - 1)

	_rent.disabled = sections.is_empty()
	_return_preview.disabled = sections.is_empty()
	_designate.disabled = rooms.is_empty()
	_unlock.disabled = bool(frame.get("lift_unlocked", false))
	_build_lift.disabled = _stops.size() < 2
	_coffee.disabled = _selected_person < 0 or kiosks.is_empty()


func show_preview(action: Dictionary, result: Dictionary) -> void:
	clear_preview()
	_preview.show()
	if not result.get("ok", false):
		_preview.text = result.get("error", "Die Vorschau konnte nicht erstellt werden.")
		return
	var impact: Dictionary = result.impact
	if action.kind != "return_section":
		_preview.text = "Einmalkosten %d" % impact.immediate_cost
		return

	var remaining: PackedStringArray = []
	for stop_id in impact.remaining_stops:
		remaining.append(str(int(stop_id)))
	var stops := ", ".join(remaining) if not remaining.is_empty() else "keine"
	_preview.text = (
		"Abschnitt %d zurückgeben\nMietänderung %+d · Einmalkosten %d\n"
		+ "Ausziehende Haushalte %d · verlorene Arbeitsplätze %d\n"
		+ "Betroffene Personen %d · verbleibende Halte %s"
	) % [
		action.section, impact.section_rent_change, impact.immediate_cost,
		impact.households_leaving.size(), impact.jobs_lost.size(),
		impact.affected_people.size(), stops,
	]
	var confirmation: Variant = impact.get("confirmation")
	if confirmation is String and not confirmation.is_empty():
		_pending_return = action.duplicate()
		_pending_return["confirmation"] = confirmation
		_confirm.show()


func clear_preview() -> void:
	_pending_return.clear()
	_preview.hide()
	_confirm.hide()


func _rent_section() -> void:
	action_requested.emit({"kind": "rent_section", "section": _selected(_section)})


func _preview_return() -> void:
	action_requested.emit({"kind": "return_section", "section": _selected(_section)})


func _confirm_return() -> void:
	var action := _pending_return.duplicate()
	clear_preview()
	action_requested.emit(action)


func _designate_room() -> void:
	action_requested.emit(
		{"kind": "designate", "place": _selected(_room), "use": _selected(_room_use)}
	)


func _unlock_lift() -> void:
	action_requested.emit({"kind": "unlock_lift"})


func _request_lift() -> void:
	var first: int = _selected(_first_stop)
	var last: int = _selected(_last_stop)
	var endpoints := _stops.filter(
		func(stop: Dictionary) -> bool: return stop.id == first or stop.id == last
	)
	var stop_ids := [first]
	if endpoints.size() == 2:
		endpoints.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a.floor < b.floor)
		var from: Dictionary = endpoints[0]
		var to: Dictionary = endpoints[1]
		stop_ids = [int(from.id)]
		var range_stops := _stops.filter(
			func(stop: Dictionary) -> bool:
				return (
					stop.column == from.column
					and stop.floor > mini(int(from.floor), int(to.floor))
					and stop.floor < maxi(int(from.floor), int(to.floor))
				)
		)
		range_stops.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a.floor < b.floor)
		for stop in range_stops:
			stop_ids.append(int(stop.id))
		stop_ids.append(int(to.id))
	action_requested.emit({"kind": "build_lift", "stops": stop_ids})


func _send_to_kiosk() -> void:
	action_requested.emit(
		{"kind": "coffee_trip", "person": _selected_person, "kiosk": _selected(_kiosk)}
	)


func _set_choices(picker: OptionButton, choices: Array) -> bool:
	if _choices.get(picker) == choices:
		return false
	var was_selected: Variant = _selected(picker)
	var had_selection := picker.selected >= 0
	_choices[picker] = choices
	picker.clear()

	if choices.is_empty():
		picker.add_item(picker.tooltip_text)
		picker.disabled = true
		return not had_selection
	picker.disabled = false

	var selected_index := 0
	var found := false
	for choice in choices:
		_add_item(picker, choice[0], choice[1])
		if choice[1] == was_selected:
			selected_index = picker.item_count - 1
			found = true
	picker.select(selected_index)
	return not found


func _add_item(picker: OptionButton, title: String, metadata: Variant) -> void:
	picker.add_item(title)
	picker.set_item_metadata(picker.item_count - 1, metadata)


func _selected(picker: OptionButton) -> Variant:
	return picker.get_selected_metadata() if picker.selected >= 0 else null


func _floor_name(floor_number: int) -> String:
	return "EG" if floor_number == 0 else "%d. OG" % floor_number


func _heading(title: String) -> void:
	var label := Label.new()
	label.text = title
	label.add_theme_font_size_override("font_size", 17)
	label.add_theme_color_override("font_color", Color("f4ddae"))
	add_child(label)


func _picker(title: String, parent: Container = null) -> OptionButton:
	var picker := OptionButton.new()
	picker.tooltip_text = title
	picker.fit_to_longest_item = false
	picker.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	(parent if parent != null else self).add_child(picker)
	return picker


func _button(title: String, callback: Callable, parent: Container = null) -> Button:
	var button := Button.new()
	button.text = title
	button.pressed.connect(callback)
	(parent if parent != null else self).add_child(button)
	return button
