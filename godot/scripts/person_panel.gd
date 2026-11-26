extends VBoxContainer

signal person_requested(person_id: int)
signal follow_changed(enabled: bool)

@onready var _people: OptionButton = $Person
@onready var _details: Label = $Details
@onready var _follow: CheckButton = $Follow

var _listed_ids := PackedInt64Array()
var _selected_id := -1


func _ready() -> void:
	_people.item_selected.connect(
		func(index: int) -> void: person_requested.emit(int(_people.get_item_metadata(index)))
	)
	_follow.toggled.connect(func(enabled: bool) -> void: follow_changed.emit(enabled))
	present(-1, {})


func refresh_people(people: Array) -> void:
	var ids := PackedInt64Array()
	for person in people:
		ids.append(int(person.id))
	if _selected_id >= 0 and not ids.has(_selected_id):
		ids.append(_selected_id)
	if ids == _listed_ids and _people.item_count > 0:
		return
	_listed_ids = ids
	_people.clear()
	_people.add_item("Person auswählen")
	_people.set_item_metadata(0, -1)
	for person_id in ids:
		_people.add_item("Person %d" % person_id)
		_people.set_item_metadata(_people.item_count - 1, person_id)
	_select_list_item(_selected_id)


func present(person_id: int, record: Dictionary) -> void:
	_selected_id = person_id
	_select_list_item(person_id)
	_follow.disabled = not record.has("id")
	_details.hide()
	if person_id < 0:
		return
	_details.show()
	if not record.has("id"):
		_details.text = "Person %d ist ausgezogen." % person_id
		return

	var location: Dictionary = record.location
	var anchor: Dictionary = location.get("place", location.get("from", {}))
	var location_text := _location_text(location.kind, int(anchor.get("floor", 0)))
	if location.kind == "at":
		location_text += " · %s" % _place_name(anchor)
	elif location.kind == "waiting_lift":
		location_text += " · Halt H%d" % int(location.stop)
	var destination: Variant = record.get("destination")
	if destination is Dictionary:
		location_text += " → %s" % _place_name(destination)
	var work_status := {"at_work": "Arbeitet", "at_home": "Zu Hause"}
	var reason: String = work_status.get(record.get("reason", ""), "")
	var reasons: Array[String] = []
	if not reason.is_empty():
		reasons.append(reason)
	if int(record.get("waited_ticks", 0)) > 0:
		reasons.append("Wartezeit %d Ticks" % record.waited_ticks)
	if int(record.get("frustration", 0)) > 0:
		reasons.append("Frust %d" % record.frustration)
	if int(record.get("dissatisfaction", 0)) > 0:
		reasons.append("Unzufriedenheit %d" % record.dissatisfaction)
	_details.text = "Haushalt %d · %s" % [record.household, location_text]
	if not reasons.is_empty():
		_details.text += "\n" + " · ".join(reasons)


func set_following(enabled: bool) -> void:
	_follow.set_pressed_no_signal(enabled)


func _select_list_item(person_id: int) -> void:
	var position := _listed_ids.find(person_id) + 1
	if position < _people.item_count:
		_people.select(position)


func _location_text(location_kind: String, floor_number: int) -> String:
	var floor_name := "EG" if floor_number == 0 else "%d. OG" % floor_number
	match location_kind:
		"walking":
			return "Unterwegs · " + floor_name
		"waiting_lift":
			return "Wartet am Lift · " + floor_name
		"riding_lift":
			return "Im Lift · " + floor_name
		_:
			return floor_name


func _place_name(place: Dictionary) -> String:
	var uses := {
		"home": "Wohnung", "office": "Büro", "kiosk": "Kiosk", "reception": "Rezeption",
		"landing": "Treppe", "corridor": "Flur", "vacant": "Leerfläche",
	}
	var name_key: Variant = place.get("use")
	if name_key == null:
		name_key = place.get("kind", "")
	return uses.get(name_key, "Etage %d" % place.get("floor", 0))
