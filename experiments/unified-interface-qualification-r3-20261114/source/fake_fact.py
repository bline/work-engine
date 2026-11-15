#!/usr/bin/python3
"""Controlled local-I/O boundary, selected and pinned only by isolated subjects."""
import json,pathlib,sys,time,os
root=pathlib.Path(__file__).resolve().parent;cfg=json.loads((root/'scenario.json').read_text());name=pathlib.Path(sys.argv[1]).name
with (root/'fact.log').open('a') as f:f.write(json.dumps(dict(name=name,pid=os.getpid(),ppid=os.getppid(),at=time.monotonic()))+'\n')
if name in cfg.get('fail_fact',[]):raise SystemExit(73)
if name=='create-argv.json':time.sleep(cfg.get('pre_tw_delay',0))
if name=='watchdog-terminal-fact.json':time.sleep(cfg.get('watch_local_delay',0))
if name=='watchdog-start.json':time.sleep(cfg.get('watch_start_delay',0))

if name=='controller-terminal-fact.json':time.sleep(cfg.get('controller_terminal_delay',0))

if name=='create-argv.json' and cfg.get('block_budget_publication'):
 p=pathlib.Path(sys.argv[1]).parent/'local-budget.json'
 if p.exists():p.unlink()
 p.mkdir()
