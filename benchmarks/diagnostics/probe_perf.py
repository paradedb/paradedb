"""Select a working perf sampling event before spending time building PostgreSQL."""
import json
from pathlib import Path
import subprocess

OUT = Path('diagnostic-output')
OUT.mkdir(exist_ok=True)
attempts = []
selected = None
# The previous runner rejected software cpu-clock:u sampling. Do not assume
# another event works: require an actual busy-process sample from the probe.
for event in ('cpu-clock:u', 'cpu-clock', 'cpu-clock:ukhHG', 'cycles:u', 'cycles', 'task-clock'):
    label = event.replace(':', '-')
    data = OUT / f'probe-{label}.data'
    command = ['sudo', 'perf', 'record', '-a', '-e', event, '-F', '199',
               '--call-graph', 'dwarf,16384', '-o', str(data), '--', 'python3', '-c',
               'import time; stop=time.monotonic()+1.0\nwhile time.monotonic()<stop: pass']
    result = subprocess.run(command, text=True, capture_output=True)
    (OUT / f'probe-{label}.log').write_text(result.stdout + result.stderr)
    script = subprocess.run(['sudo', 'perf', 'script', '-i', str(data)],
                            text=True, capture_output=True) if result.returncode == 0 else None
    valid = script is not None and script.returncode == 0 and 'python3' in script.stdout
    attempts.append({'event': event, 'command': command, 'returncode': result.returncode, 'has_python_samples': valid})
    (OUT / 'perf-probe-attempts.json').write_text(json.dumps(attempts, indent=2))
    if valid:
        (OUT / 'perf-probe-stacks.txt').write_text(script.stdout)
        selected = event
        break
assert selected is not None, 'No event produced CPU samples; see perf-probe-attempts.json and probe logs'
result = subprocess.run(['sudo', 'perf', 'sched', 'record', '-a', '-o', str(OUT / 'probe-sched.data'),
                         '--', 'sleep', '0.1'], text=True, capture_output=True)
(OUT / 'perf-sched-probe.log').write_text(result.stdout + result.stderr)
(OUT / 'perf-capability.json').write_text(json.dumps({'event': selected, 'scheduler': result.returncode == 0}, indent=2))
print(f'CPU event: {selected}; scheduler trace available: {result.returncode == 0}', flush=True)
