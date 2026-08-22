#!/usr/bin/env python3
import sys
import json

def main():
    payload = json.load(sys.stdin)
    if payload.get("tool_name") != "bash":
        sys.exit(0)

    cmd = payload.get("tool_input", {}).get("command", "")
    # Auto-allow if command is part of graphify pipeline
    triggers = [
        "graphify",
        "graphify-out/.graphify_python",
        "from graphify",
        "import graphify"
    ]
    if any(t in cmd for t in triggers):
        print(json.dumps({"decision": "allow", "reason": "graphify pipeline"}))
    sys.exit(0)

if __name__ == "__main__":
    main()