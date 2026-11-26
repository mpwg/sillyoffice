extends SceneTree

const STORE_SCRIPT := "res://scripts/save_store.gd"
const OLD := '{"generation":1,"ort":"Büro"}'
const CURRENT := '{"generation":2,"ort":"Büro"}'
const NEXT := '{"generation":3,"ort":"Büro"}'
const INVALID := '{"generation":'


func _initialize() -> void:
	quit(run())


static func run() -> int:
	if not ResourceLoader.exists(STORE_SCRIPT):
		return _fail("SaveStore fehlt. Die Wiederherstellung kann noch nicht geprüft werden.")
	var script := load(STORE_SCRIPT) as GDScript
	if script == null or not script.can_instantiate():
		return _fail("SaveStore kann nicht geladen werden.")
	var directory := DirAccess.create_temp("sillyoffice-save-store")
	if directory == null:
		return _fail("Temporäres Testverzeichnis kann nicht angelegt werden.")
	var root := directory.get_current_dir()
	var result := _test_successive_saves(script, root.path_join("rotation.json"))
	if result.is_empty():
		result = _test_interrupted_saves(script, root)
	if result.is_empty():
		result = _test_blocked_rotation(script, root.path_join("blockiert.json"))
	if not result.is_empty():
		return _fail(result)
	print("SaveStore geprüft. Spielstände bleiben nach unterbrochenem Schreiben und Austausch lesbar.")
	return 0


static func _test_successive_saves(script: GDScript, path: String) -> String:
	var store = script.new()
	for contents in [OLD, CURRENT, NEXT]:
		var result: String = store.save(path, contents, _valid)
		if not result.is_empty():
			return "Spielstand konnte nicht gespeichert werden. " + result
	if store.load_candidates(path) != PackedStringArray([NEXT, CURRENT]):
		return "Hauptdatei und unmittelbare Vorgängergeneration fehlen."
	if store.save(path, INVALID, _valid).is_empty():
		return "Ein unvollständiger Spielstand wurde angenommen."
	if store.load_candidates(path) != PackedStringArray([NEXT, CURRENT]):
		return "Ein abgelehnter Spielstand hat vorhandene Generationen verändert."
	if FileAccess.file_exists(path + ".tmp"):
		return "Eine abgeschlossene oder abgelehnte Sicherung hinterlässt eine Temporärdatei."
	return ""


static func _test_interrupted_saves(script: GDScript, root: String) -> String:
	var interruptions := [
		["schreiben", CURRENT, OLD, INVALID, CURRENT],
		["vor-rotation", CURRENT, OLD, NEXT, CURRENT],
		["vor-rotation-ohne-vorgänger", CURRENT, "", NEXT, CURRENT],
		["nach-rotation", "", CURRENT, NEXT, CURRENT],
		["nach-installation", NEXT, CURRENT, "", NEXT],
		["beschädigte-hauptdatei", INVALID, CURRENT, "", CURRENT],
	]
	for fixture in interruptions:
		var name: String = fixture[0]
		var path := root.path_join(name + ".json")
		for index in range(3):
			var suffix: String = ["", ".previous", ".tmp"][index]
			var contents: String = fixture[index + 1]
			if not contents.is_empty() and not _write_file(path + suffix, contents):
				return "Testdatei für %s konnte nicht geschrieben werden." % name
		var store = script.new()
		var recovered := ""
		for candidate in store.load_candidates(path):
			if _valid(candidate):
				recovered = candidate
				break
		if recovered != fixture[4]:
			return "Nach %s wurde die letzte gültige Generation nicht gefunden." % name
		var result: String = store.save(path, OLD, _valid)
		if not result.is_empty():
			return "Speichern nach %s schlug fehl. %s" % [name, result]
		if store.load_candidates(path) != PackedStringArray([OLD, recovered]):
			return "Speichern nach %s hat die gültige Vorgängergeneration verändert." % name
	return ""


static func _test_blocked_rotation(script: GDScript, path: String) -> String:
	var store = script.new()
	var result: String = store.save(path, CURRENT, _valid)
	if not result.is_empty():
		return "Vorbereitung der blockierten Rotation schlug fehl. " + result
	if DirAccess.make_dir_absolute(path + ".previous") != OK:
		return "Das Testverzeichnis für die blockierte Rotation fehlt."
	if not _write_file(path + ".previous/inhalt", OLD):
		return "Blockierende Testdatei fehlt."
	if store.save(path, NEXT, _valid).is_empty():
		return "Der blockierte Dateiaustausch wurde als erfolgreich gemeldet."
	if store.load_candidates(path) != PackedStringArray([CURRENT]):
		return "Ein fehlgeschlagener Dateiaustausch hat den Hauptstand verändert."
	return ""


static func _write_file(path: String, contents: String) -> bool:
	var file := FileAccess.open(path, FileAccess.WRITE)
	return file != null and file.store_string(contents)


static func _valid(contents: String) -> bool:
	return contents in [OLD, CURRENT, NEXT]


static func _fail(message: String) -> int:
	printerr(message)
	return 1
