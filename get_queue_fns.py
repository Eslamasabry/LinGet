with open("src/cli/tui/ui.rs", "r") as f:
    lines = f.readlines()

def get_fn_bounds(lines, fn_name):
    start = -1
    end = -1
    braces = 0
    in_fn = False
    for i, line in enumerate(lines):
        if line.startswith(f"fn {fn_name}(") or line.startswith(f"pub fn {fn_name}("):
            start = i
            in_fn = True
        if in_fn:
            braces += line.count('{')
            braces -= line.count('}')
            if braces == 0:
                end = i + 1
                break
    return start, end

# Let's find what other functions exist after 476.
for i in range(476, len(lines)):
    if lines[i].startswith("fn ") or lines[i].startswith("pub fn "):
        print(f"{i}: {lines[i].strip()}")

