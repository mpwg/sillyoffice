extends RefCounted

const PREVIOUS_SUFFIX := ".previous"
const TEMPORARY_SUFFIX := ".tmp"


func save(path: String, contents: String, validate: Callable) -> String:
	if not validate.is_valid() or not validate.call(contents):
		return "Der neue Spielstand konnte nicht als gültig bestätigt werden."
	var primary := ProjectSettings.globalize_path(path)
	var previous := primary + PREVIOUS_SUFFIX
	var temporary := primary + TEMPORARY_SUFFIX
	var encoded := contents.to_utf8_buffer()
	var file := FileAccess.open(temporary, FileAccess.WRITE)
	if file == null:
		return "Temporäre Spielstanddatei kann nicht geöffnet werden (%s)." % error_string(FileAccess.get_open_error())
	var written := file.store_buffer(encoded)
	file.flush()
	var write_error := file.get_error()
	file.close()
	if not written or write_error != OK or _read(temporary) != encoded:
		return "Temporäre Spielstanddatei konnte nicht vollständig geschrieben werden."

	if FileAccess.file_exists(primary):
		var stored: Variant = _read(primary)
		if stored == null:
			return "Der vorhandene Spielstand kann nicht vollständig gelesen werden."
		# Nur ein gültiger Hauptstand darf die Vorgängergeneration ersetzen.
		if validate.call(stored.get_string_from_utf8()):
			var rotation_error := DirAccess.rename_absolute(primary, previous)
			if rotation_error != OK:
				return "Vorgängergeneration kann nicht gesichert werden (%s)." % error_string(rotation_error)

	var install_error := DirAccess.rename_absolute(temporary, primary)
	if install_error != OK:
		return "Der neue Spielstand kann nicht übernommen werden (%s)." % error_string(install_error)
	return ""


func load_candidates(path: String) -> PackedStringArray:
	var primary := ProjectSettings.globalize_path(path)
	var candidates := PackedStringArray()
	for candidate in [primary, primary + PREVIOUS_SUFFIX]:
		var contents: Variant = _read(candidate)
		if contents != null:
			candidates.append(contents.get_string_from_utf8())
	return candidates


func _read(path: String) -> Variant:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		return null
	var size := file.get_length()
	var contents := file.get_buffer(size)
	if contents.size() != size:
		return null
	return contents
