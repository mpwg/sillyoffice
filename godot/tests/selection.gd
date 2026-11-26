extends RefCounted


static func run(partie) -> int:
	if not partie.has_method("selected_person_id"):
		return _fail("Die Partie speichert die ausgewählte Personen-ID noch nicht.")
	await partie.get_tree().process_frame
	var building = partie.get_node("Layout/Body/Building")
	var panel = partie.get_node("Layout/Body/Sidebar/Content/PersonPanel")
	var original := _frame(29, 0)
	building.fit_columns(original.places, original.sections)
	building.present(original, 0, 0, 0.0)

	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	click.position = (
		building.global_position + building._person_point(original.people[0]) + Vector2(0.0, -18.0)
	)
	click.global_position = click.position
	partie.get_viewport().push_input(click, true)
	if partie.selected_person_id() != 29:
		return _fail("Der Klick wählte die Person im Gebäudeschnitt nicht aus.")

	building.present(_frame(71, 1), 1, 1, 0.0)
	if partie.selected_person_id() != 29:
		return _fail("Beim Wegschwenken wechselte die Auswahl zu einer anderen Person.")
	building.present(original, 0, 0, 0.0)
	if partie.selected_person_id() != 29:
		return _fail("Beim Zurückschwenken ging die Personen-ID verloren.")

	panel.present(29, {})
	if not panel.get_node("Details").text.contains("ausgezogen"):
		return _fail("Die Auskunft meldet den Auszug der ausgewählten Person nicht.")
	return 0


static func _frame(person_id: int, floor_number: int) -> Dictionary:
	var place := {"id": 1, "floor": floor_number, "column": 1, "kind": "reception"}
	return {
		"places": [place],
		"sections": [],
		"people": [{"id": person_id, "location": {"kind": "at", "place": place}}],
	}


static func _fail(message: String) -> int:
	printerr(message)
	return 1
