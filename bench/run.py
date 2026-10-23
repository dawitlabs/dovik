import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PROGRAMS = ROOT / "programs"
BUILD = ROOT / "build"
COMPILER = ROOT.parent / "compiler"
DOVIK = COMPILER / "target" / "debug" / "dovik"
WASI_HOST = COMPILER / "tests" / "wasi-host.mjs"

WORKLOADS = [
    "arith",
    "recursion",
    "calls",
    "collections",
    "strings",
    "string_index",
    "allocation",
]

REPS = int(os.environ.get("BENCH_REPS", "5"))


def wasm_available():
    if not WASI_HOST.exists() or shutil.which("node") is None:
        return False

    probe = BUILD / "probe.dovk"
    BUILD.mkdir(exist_ok=True)
    probe.write_text("print(1)\n")
    built = shell([str(DOVIK), "build", "--target", "wasm",
                   "-o", str(BUILD / "probe.wasm"), str(probe)])
    return built.returncode == 0


def shell(args, **kwargs):
    return subprocess.run(args, capture_output=True, text=True, **kwargs)


def version(args, pattern=r"(\d+\.\d+\.\d+)"):
    result = shell(args)
    text = (result.stdout or result.stderr).strip()
    found = re.search(pattern, text)
    return found.group(1) if found else text.splitlines()[0] if text else "unknown"


def environment():
    cpu = "unknown"
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass

    return {
        "cpu": cpu,
        "cores": os.cpu_count(),
        "kernel": platform.release(),
        "os": platform.system(),
        "clang": version(["clang", "--version"]),
        "rustc": version(["rustc", "--version"]),
        "python": platform.python_version(),
        "dovik": version([str(DOVIK), "--version"]),
        "reps": REPS,
    }


WASM = False


def build(workload):
    BUILD.mkdir(exist_ok=True)
    results = {}

    targets = {
        "dovik": (
            [str(DOVIK), "build", "-o", str(BUILD / f"{workload}.dovik"),
             str(PROGRAMS / f"{workload}.dovk")],
            BUILD / f"{workload}.dovik",
        ),
        "dovik-unchecked": (
            [str(DOVIK), "build", "--omit-runtime-checks-for-benchmarking",
             "-o", str(BUILD / f"{workload}.unchecked"),
             str(PROGRAMS / f"{workload}.dovk")],
            BUILD / f"{workload}.unchecked",
        ),
        "c": (
            ["clang", "-O2", "-o", str(BUILD / f"{workload}.c.out"),
             str(PROGRAMS / f"{workload}.c")],
            BUILD / f"{workload}.c.out",
        ),
        "rust": (
            ["rustc", "-O", "-o", str(BUILD / f"{workload}.rs.out"),
             str(PROGRAMS / f"{workload}.rs")],
            BUILD / f"{workload}.rs.out",
        ),
    }

    if WASM:
        targets["dovik-wasm"] = (
            [str(DOVIK), "build", "--target", "wasm",
             "-o", str(BUILD / f"{workload}.wasm"),
             str(PROGRAMS / f"{workload}.dovk")],
            BUILD / f"{workload}.wasm",
        )

    for name, (command, binary) in targets.items():
        start = time.perf_counter()
        result = shell(command)
        elapsed = time.perf_counter() - start

        if result.returncode != 0:
            print(f"  {workload}/{name}: build failed\n{result.stderr}", file=sys.stderr)
            continue

        run = [str(binary)]

        if name == "dovik-wasm":
            run = ["node", "--no-warnings", str(WASI_HOST), str(binary)]

        results[name] = {
            "compile_seconds": elapsed,
            "binary_bytes": binary.stat().st_size if binary.exists() else None,
            "command": run,
        }

    results["python"] = {
        "compile_seconds": None,
        "binary_bytes": None,
        "command": [sys.executable, str(PROGRAMS / f"{workload}.py")],
    }

    return results


def high_water(pid, out):
    status = Path(f"/proc/{pid}/status")

    while True:
        try:
            for line in status.read_text().splitlines():
                if line.startswith("VmHWM"):
                    out[0] = max(out[0], int(line.split()[1]))
                    break
        except (OSError, ValueError):
            return
        time.sleep(0.0005)


def time_once(command):
    start = time.perf_counter()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    stdout = process.stdout.read().decode()
    process.stdout.close()
    code = process.wait()
    return time.perf_counter() - start, stdout, code


def memory_once(command):
    process = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    observed = [0]
    watcher = threading.Thread(target=high_water, args=(process.pid, observed), daemon=True)
    watcher.start()
    process.wait()
    watcher.join(timeout=0.05)
    return observed[0]


def measure(command):
    samples = []
    stdout = ""

    for _ in range(REPS):
        elapsed, stdout, code = time_once(command)

        if code != 0:
            return None

        samples.append(elapsed)

    peak = max(memory_once(command) for _ in range(2))

    return {
        "best_seconds": min(samples),
        "median_seconds": statistics.median(samples),
        "peak_rss_kb": peak,
        "stdout": stdout.strip(),
    }


def main():
    global WASM

    if not DOVIK.exists():
        print("build the compiler first: cargo build", file=sys.stderr)
        return 1

    WASM = wasm_available()
    report = {"environment": environment(), "workloads": {}}
    print(f"environment: {report['environment']['cpu']}, {REPS} reps per measurement")
    print("peak RSS is absolute VmHWM, sampled; runs under ~20ms may under-report")

    if WASM:
        print("dovik-wasm runs under node, so its time includes host startup and its")
        print("peak RSS is node's, not the module's. Compare it to dovik, not to c.\n")
    else:
        print("dovik-wasm skipped: no wasi sysroot in this build, or no node\n")

    for workload in WORKLOADS:
        print(f"{workload}:")
        builds = build(workload)
        entries = {}
        outputs = {}

        for name, info in builds.items():
            measured = measure(info["command"])

            if measured is None:
                print(f"  {name}: run failed")
                continue

            outputs[name] = measured.pop("stdout")
            measured["compile_seconds"] = info["compile_seconds"]
            measured["binary_bytes"] = info["binary_bytes"]
            entries[name] = measured
            print(
                f"  {name:16} {measured['best_seconds']:8.4f}s"
                f"  {measured['peak_rss_kb']:>9} KB"
            )

        distinct = set(outputs.values())
        if len(distinct) > 1:
            print(f"  WARNING: implementations disagree: {outputs}")
            entries["_agreement"] = False
        else:
            entries["_agreement"] = True

        report["workloads"][workload] = entries
        print()

    (ROOT / "results.json").write_text(json.dumps(report, indent=2))
    print(f"wrote {ROOT / 'results.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
