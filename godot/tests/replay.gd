extends RefCounted

const SCENARIO := "res://content/zwei-etagen.json"
const REPLAY := "res://content/erster-weg.json"


static func run(partie, args: PackedStringArray) -> int:
	if args.size() < 2 or args[1] != "erster-weg":
		return _fail("Unbekannter Replay-Ablauf. Erwartet wird erster-weg.")
	var output_path := ""
	var reference_path := ""
	var alternate_camera := false
	var index := 2
	while index < args.size():
		var option := args[index]
		index += 1
		match option:
			"--out", "--reference":
				if index == args.size() or args[index].begins_with("--"):
					return _fail("Dateipfad fehlt für %s." % option)
				if option == "--out":
					output_path = args[index]
				else:
					reference_path = args[index]
				index += 1
			"--camera-alternate":
				alternate_camera = true
			_:
				return _fail("Unbekannte Replay-Option %s." % option)
	if output_path.is_empty():
		return _fail("Ausgabedatei fehlt. Verwende --out DATEI.")
	if not partie.has_method("start_partie"):
		return _fail("Die Startszene besitzt keine Partienanbindung.")
	var load_error: String = partie.start_partie(SCENARIO, REPLAY)
	if not load_error.is_empty():
		return _fail(load_error)

	var replay: Variant = JSON.parse_string(FileAccess.get_file_as_string(REPLAY))
	if replay is not Dictionary or replay.get("checkpoints") is not Array:
		return _fail("Kontrollticks fehlen im Replay-Ablauf.")
	var reference: Array = []
	if not reference_path.is_empty():
		var expected: Variant = JSON.parse_string(FileAccess.get_file_as_string(reference_path))
		if expected is not Dictionary or expected.get("checkpoints") is not Array:
			return _fail("CLI-Referenz ist unlesbar oder enthält keine Kontrollpunkte.")
		reference = expected.checkpoints

	partie.set_paused(true)
	var paused_tick: float = partie.render_tick()
	await partie.get_tree().create_timer(0.2).timeout
	if partie.render_tick() != paused_tick:
		return _fail("Die Spielzeit lief während der Pause weiter.")

	var checkpoints: PackedStringArray = []
	var tick := 0
	for target in replay.checkpoints:
		while tick < int(target):
			var floor_number := (tick + int(alternate_camera)) % 2
			var before_camera: float = partie.render_tick()
			var frame: Dictionary = partie.look_at_floor(floor_number)
			if frame.get("tick", -1) != tick or partie.render_tick() != before_camera:
				return _fail("Kameraschwenk bei Tick %d veränderte die Spielzeit." % tick)
			var places: Array = frame.get("places", [])
			if places.is_empty():
				return _fail(
					"Die Kamera zeigt bei Tick %d keine Orte auf Etage %d." % [tick, floor_number]
				)
			for place in places:
				if place.floor != floor_number:
					return _fail(
						"Die Kamera zeigt bei Tick %d einen Ort außerhalb von Etage %d."
						% [tick, floor_number]
					)
			if not partie.advance_tick():
				return _fail("Der nächste Tick nach %d konnte nicht ausgeführt werden." % tick)
			tick += 1
		var serialized: String = partie.checkpoint_json()
		var actual: Variant = JSON.parse_string(serialized)
		if actual is not Dictionary or actual.get("tick", -1) != tick:
			return _fail(
				"Ungültiger Kontrollpunkt für Tick %d. Godot meldet %s" % [tick, serialized]
			)
		if not reference_path.is_empty():
			var expected: Variant = null
			if checkpoints.size() < reference.size():
				expected = reference[checkpoints.size()]
			if actual != expected:
				return _difference(tick, expected, actual)
		checkpoints.append(serialized)
	if not reference_path.is_empty() and checkpoints.size() < reference.size():
		var expected: Variant = reference[checkpoints.size()]
		var expected_tick := int(expected.get("tick", tick)) if expected is Dictionary else tick
		return _difference(expected_tick, expected, null)
	var office: Dictionary = partie.look_at_floor(1)
	var people: Array = office.get("people", [])
	if people.size() != 1 or people[0].id != 1 or people[0].location.kind != "at":
		return _fail("Person 1 ist bei Tick %d im Büro nicht sichtbar." % tick)
	if people[0].location.place.kind != "office":
		return _fail("Person 1 steht bei Tick %d nicht an der Bürotür." % tick)
	var output := FileAccess.open(output_path, FileAccess.WRITE)
	if output == null:
		return _fail(
			"Replay-Bericht kann nicht geöffnet werden (%s)."
			% error_string(FileAccess.get_open_error())
		)
	if not output.store_string("{\"checkpoints\":[%s]}\n" % ",".join(checkpoints)):
		return _fail(
			"Replay-Bericht kann nicht geschrieben werden (%s)." % error_string(output.get_error())
		)
	return 0


static func _difference(tick: int, expected: Variant, actual: Variant) -> int:
	return _fail(
		"Abweichung bei Tick %d.\nCLI %s\nGodot %s"
		% [tick, JSON.stringify(expected), JSON.stringify(actual)]
	)


static func _fail(message: String) -> int:
	printerr(message)
	return 1
