import json
import math
import re
import tomllib

SCHEMA_LINE = "#:schema https://raw.githubusercontent.com/Zira3l137/CharPaper/main/schemas/suite.schema.json"
ORDER = ("character", "animations", "environment", "environments", "post", "camera", "cameras", "gaze")
BARE_KEY = re.compile(r"^[A-Za-z0-9_-]+$")


def load(path):
    with open(path, "rb") as file:
        return tomllib.load(file)


def _key(key):
    return key if BARE_KEY.match(key) else json.dumps(key, ensure_ascii=False)


def _value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        if math.isnan(value):
            return "nan"
        if math.isinf(value):
            return "inf" if value > 0 else "-inf"
        return repr(round(value, 6))
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, (list, tuple)):
        return "[" + ", ".join(_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{_key(k)} = {_value(v)}" for k, v in value.items()) + " }"
    if hasattr(value, "isoformat"):
        return value.isoformat()
    raise TypeError(f"cannot write {type(value).__name__} to TOML")


def _is_table(value):
    return isinstance(value, dict)


def _is_table_array(value):
    return isinstance(value, list) and value and all(isinstance(item, dict) for item in value)


def _table(lines, path, table):
    scalars = [(k, v) for k, v in table.items() if not _is_table(v) and not _is_table_array(v)]
    if scalars or not table:
        lines.append("")
        lines.append("[" + ".".join(_key(p) for p in path) + "]")
        lines.extend(f"{_key(k)} = {_value(v)}" for k, v in scalars)
    for key, value in table.items():
        if _is_table(value):
            _table(lines, path + [key], value)
        elif _is_table_array(value):
            for item in value:
                lines.append("")
                lines.append("[[" + ".".join(_key(p) for p in path + [key]) + "]]")
                lines.extend(f"{_key(k)} = {_value(v)}" for k, v in item.items())


def dumps(data):
    lines = [SCHEMA_LINE, ""]
    for key, value in data.items():
        if not _is_table(value) and not _is_table_array(value):
            lines.append(f"{_key(key)} = {_value(value)}")
    tables = [k for k in ORDER if k in data] + [
        k for k in data if k not in ORDER and (_is_table(data[k]) or _is_table_array(data[k]))
    ]
    for key in tables:
        value = data[key]
        if _is_table(value):
            if value:
                _table(lines, [key], value)
        else:
            for item in value:
                lines.append("")
                lines.append(f"[[{_key(key)}]]")
                lines.extend(f"{_key(k)} = {_value(v)}" for k, v in item.items())
    return "\n".join(lines) + "\n"
