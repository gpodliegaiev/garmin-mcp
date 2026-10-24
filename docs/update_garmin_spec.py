#!/usr/bin/env python3
"""Refresh the local copy of Garmin's personal API docs.

Garmin's swagger page (SWAGGER_PAGE) builds itself in the browser, so the
OpenAPI spec isn't a separate file: it is embedded in the page's JavaScript
bundle as `JSON.parse('{"openapi":...}')`. This script downloads that bundle,
pulls the spec out, and writes:

- docs/garmin-openapi.json  the full spec (open in any OpenAPI viewer)
- docs/garmin-api.md        a readable index: endpoints, params, responses,
                            and which endpoints this server already calls

Implementation status is found by scanning src/ for the endpoint paths, so it
never needs editing by hand. For each path it shows the `GarminClient` method
that calls it and the MCP tool that uses that method.

Usage (standard library only):
  python3 docs/update_garmin_spec.py            download the spec, write both
  python3 docs/update_garmin_spec.py --offline  rebuild garmin-api.md from the
                                                local spec (after adding a tool)
"""
import json
import pathlib
import re
import sys
import urllib.request

SITE = "https://developer.garmin.com"
SWAGGER_PAGE = SITE + "/gc-developer-program/personal/swagger-docs/"
DOCS = pathlib.Path(__file__).resolve().parent
SRC = DOCS.parent / "src"
METHODS = ("get", "post", "put", "patch", "delete")


def fetch(url):
    """Downloads `url` and returns the body as text."""
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read().decode("utf-8")


def find_bundle(html):
    """Returns the URL of the page's JS bundle (its name has a content hash)."""
    match = re.search(
        r'src="(/component---src-pages-gc-developer-program-personal-swagger-docs-js-[0-9a-f]+\.js)"',
        html,
    )
    if not match:
        raise SystemExit("swagger page bundle not found; Garmin changed the page layout")
    return SITE + match.group(1)


def extract_spec(js):
    """Decodes the single-quoted JS string passed to JSON.parse and parses it."""
    marker = "JSON.parse('{\"openapi\""
    start = js.find(marker)
    if start < 0:
        raise SystemExit("embedded OpenAPI spec not found in bundle")
    i = start + len("JSON.parse('")
    escapes = {"n": "\n", "t": "\t", "r": "\r", "b": "\b", "f": "\f",
               "'": "'", '"': '"', "\\": "\\", "/": "/"}
    out = []
    # Read up to the closing unescaped quote, undoing JS string escapes.
    while js[i] != "'":
        if js[i] != "\\":
            out.append(js[i])
            i += 1
        elif js[i + 1] == "u":
            out.append(chr(int(js[i + 2:i + 6], 16)))
            i += 6
        elif js[i + 1] == "x":
            out.append(chr(int(js[i + 2:i + 4], 16)))
            i += 4
        else:
            out.append(escapes[js[i + 1]])
            i += 2
    return json.loads("".join(out))


def type_name(schema):
    """Short type label: a schema name for `$ref`, `X[]` for arrays."""
    if not schema:
        return ""
    if "$ref" in schema:
        return schema["$ref"].split("/")[-1]
    kind = schema.get("type", "")
    if kind == "array":
        return type_name(schema.get("items", {})) + "[]"
    if schema.get("format"):
        kind += f" ({schema['format']})"
    if "enum" in schema:
        kind += ": " + ", ".join(map(str, schema["enum"]))
    return kind


def cell(value):
    """Makes text safe for one markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def operations(spec):
    """Yields (METHOD, path, operation) for every endpoint in the spec."""
    for path, ops in spec["paths"].items():
        for method, op in ops.items():
            if method in METHODS:
                yield method.upper(), path, op


def anchor(method, path):
    """GitHub-style heading anchor for `## METHOD path`."""
    return re.sub(r"[^a-z0-9 _-]", "", f"{method} {path}".lower()).replace(" ", "-")


def enclosing_fns(text, pattern):
    """Names of the Rust `fn`s whose body has a match for `pattern`.

    Takes the nearest `fn name` above each match, which is enough for this
    codebase (no nested functions around API calls).
    """
    names = []
    current = None
    for line in text.splitlines():
        found = re.search(r"\bfn (\w+)", line)
        if found:
            current = found.group(1)
        if current and re.search(pattern, line) and current not in names:
            names.append(current)
    return names


def implemented(spec):
    """Maps each spec path called by the server to `(client method, tool)`.

    Finds string literals starting with `/` in src/, either a `const` (then the
    method is the `fn` that uses the constant) or inline in a `fn`. Spec
    placeholders like `{activityId}` match any `{...}` in the Rust string, so
    `format!("/activity-service/activity/{activity_id}")` counts.
    """
    client = (SRC / "garmin" / "client.rs").read_text(encoding="utf-8")
    server = (SRC / "server.rs").read_text(encoding="utf-8")
    calls = {}  # Rust path literal -> client method name
    for line_text in client.splitlines():
        for literal in re.findall(r'"(/[^"]*)"', line_text):
            const = re.match(r"\s*(?:pub\s+)?const (\w+)", line_text)
            if const:
                methods = enclosing_fns(client, rf"\b{const.group(1)}\b")
            else:
                methods = enclosing_fns(client, re.escape(f'"{literal}"'))
            if methods:
                calls[literal] = methods[0]

    status = {}
    for path in spec["paths"]:
        pattern = "^" + re.sub(r"\\\{.*?\\\}", r"\{[^}]+\}", re.escape(path)) + "$"
        for literal, method in calls.items():
            if re.match(pattern, literal):
                tools = enclosing_fns(server, rf"\.{method}\(")
                status[path] = (method, ", ".join(f"`{t}`" for t in tools) or "none")
    return status


def markdown(spec):
    """Renders the endpoint index as markdown."""
    status = implemented(spec)
    lines = [
        "# Garmin Connect Personal API: endpoint index",
        "",
        f"Generated by `update_garmin_spec.py` from `garmin-openapi.json` "
        f"(OpenAPI {spec['openapi']}, source: {SWAGGER_PAGE}).",
        f"Base URL: `{spec['servers'][0]['url']}`. "
        "All endpoints use `Authorization: Bearer <token>`.",
        "",
        "Full request and response schemas are in `garmin-openapi.json` under `components.schemas`.",
        "",
        "## Endpoints",
        "",
        f"{len(status)} of {len(spec['paths'])} endpoints are implemented. "
        "Status comes from scanning `src/`; refresh it with "
        "`python3 docs/update_garmin_spec.py --offline`.",
        "",
        "| Endpoint | MCP tool | `GarminClient` method |",
        "|---|---|---|",
    ]
    for method, path, _ in operations(spec):
        client_fn, tool = status.get(path, ("", ""))
        link = f"[`{method} {path}`](#{anchor(method, path)})"
        if client_fn:
            lines.append(f"| ✅ {link} | {tool} | `{client_fn}` |")
        else:
            lines.append(f"| {link} | not implemented | |")
    lines.append("")

    for method, path, op in operations(spec):
        lines += [f"## {method} {path}", ""]
        if path in status:
            client_fn, tool = status[path]
            lines += [f"**Implemented:** MCP tool {tool}, via `GarminClient::{client_fn}` "
                      "(`src/garmin/client.rs`).", ""]
        else:
            lines += ["**Not implemented.**", ""]
        text = ". ".join(t for t in (op.get("summary"), op.get("description")) if t)
        if text:
            lines += [cell(text), ""]
        params = op.get("parameters", [])
        if params:
            lines += ["| Param | In | Type | Default | Description |", "|---|---|---|---|---|"]
            for p in params:
                schema = p.get("schema", {})
                name = p["name"] + (" *(required)*" if p.get("required") else "")
                lines.append(
                    f"| `{name}` | {p.get('in', '')} | {cell(type_name(schema))} "
                    f"| {cell(schema.get('default', ''))} | {cell(p.get('description', ''))} |"
                )
            lines.append("")
        for code, response in op.get("responses", {}).items():
            body = ""
            for content_type, content in (response.get("content") or {}).items():
                if content.get("schema"):
                    body = f" → `{type_name(content['schema'])}` ({content_type})"
                else:
                    body = f" ({content_type})"
            lines.append(f"- **{code}**{body}: {cell(response.get('description', ''))}")
        lines.append("")
    return "\n".join(lines)


def main():
    spec_file = DOCS / "garmin-openapi.json"
    if "--offline" in sys.argv[1:]:
        spec = json.loads(spec_file.read_text(encoding="utf-8"))
    else:
        spec = extract_spec(fetch(find_bundle(fetch(SWAGGER_PAGE))))
        spec_file.write_text(json.dumps(spec, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    (DOCS / "garmin-api.md").write_text(markdown(spec), encoding="utf-8")
    schemas = len(spec.get("components", {}).get("schemas", {}))
    done = len(implemented(spec))
    print(f"wrote {len(spec['paths'])} paths ({done} implemented), {schemas} schemas to {DOCS}")


if __name__ == "__main__":
    main()
