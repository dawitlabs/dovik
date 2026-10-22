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
RUNTIME = COMPILER / "target" / "debug" / "libdovik_rt.a"

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

    for name, (command, binary) in targets.items():
        start = time.perf_counter()
        result = shell(command, env={**os.environ, "DOVIK_RUNTIME": str(RUNTIME)})
        elapsed = time.perf_counter() - start

        if result.returncode != 0:
            print(f"  {workload}/{name}: build failed\n{result.stderr}", file=sys.stderr)
            continue

        results[name] = {
            "compile_seconds": elapsed,
            "binary_bytes": binary.stat().st_size if binary.exists() else None,
            "command": [str(binary)],
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


def measure(command):
    samples = []
    peak = 0
    stdout = ""

    for _ in range(REPS):
        start = time.perf_counter()
        process = subprocess.Popen(
            command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
        )
        observed = [0]
        watcher = threading.Thread(
            target=high_water, args=(process.pid, observed), daemon=True
        )
        watcher.start()
        stdout = process.stdout.read().decode()
        process.stdout.close()
        code = process.wait()
        elapsed = time.perf_counter() - start
        watcher.join(timeout=0.05)

        if code != 0:
            return None

        samples.append(elapsed)
        peak = max(peak, observed[0])

    return {
        "best_seconds": min(samples),
        "median_seconds": statistics.median(samples),
        "peak_rss_kb": peak,
        "stdout": stdout.strip(),
    }


def main():
    if not DOVIK.exists():
        print("build the compiler first: cargo build", file=sys.stderr)
        return 1
    if not RUNTIME.exists():
        print("build the runtime first: cargo build -p dovik-rt", file=sys.stderr)
        return 1

    report = {"environment": environment(), "workloads": {}}
    print(f"environment: {report['environment']['cpu']}, {REPS} reps per measurement")
    print("peak RSS is absolute VmHWM, sampled; runs under ~20ms may under-report\n")

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
