import json, sys, subprocess, shutil, os, time
ROOT='C:/Users/jayte/Documents/dev/SoccerManager'
S='C:/Users/jayte/AppData/Local/Temp/claude/C--Users-jayte-Documents-dev-SoccerManager/a7ca6102-5eb6-4026-b025-47fc91bca540/scratchpad/ks/runs'
BIN=ROOT+'/target/release/engine-cli.exe'
BASE=ROOT+'/.ai/workflows/football-manager-match-engine/implement-evidence/keeper-and-shots/baseline/set-base/report.json'
name=sys.argv[1]; over=json.loads(sys.argv[2]); what=sys.argv[3] if len(sys.argv)>3 else 'equal'
matches=sys.argv[4] if len(sys.argv)>4 else '200'
d=f'{S}/{name}'
os.makedirs(d, exist_ok=True)
if os.path.exists(d+'/content'): shutil.rmtree(d+'/content')
shutil.copytree(ROOT+'/content', d+'/content')
tj=json.load(open(ROOT+'/content/tuning.json'))
for k,v in over.items():
    o=tj['engine']
    parts=k.split('.')
    for p in parts[:-1]: o=o[p]
    assert parts[-1] in o, k
    o[parts[-1]]=v
json.dump(tj, open(d+'/content/tuning.json','w'), indent=2)
out=[]
def run(args, tag):
    od=f'{d}/{tag}-{int(time.time())}'
    subprocess.run([BIN,'calibrate','--content-dir',d+'/content','--out',od]+args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return json.load(open(od+'/report.json')), od
if 'equal' in what:
    args=['--suite','equal','--seed','42','--matches',matches]
    if matches=='200': args+=['--baseline',BASE]
    r,od=run(args,'equal')
    b={x['band']:x['value'] for x in r['calib.bands']}
    out.append('EQ sot=%.3f gxg=%.3f cor=%.2f gk=%.2f goals=%.2f shots=%.2f diff=%s' % (b['shots_on_target_share'],b['goals_per_xg'],b['corners_per_team'],b['goal_kicks_per_match'],b['goals_per_match'],b['shots_per_team'], (r.get('calib.diff') or {}).get('changes')))
if 'red' in what:
    r,od=run(['--suite','red-card','--seed','1','--matches','240'],'red')
    rc=r['calib.red_card']; s=r['calib.suites']['red-card']
    out.append('RED ctrl=%s lim=%.2f '%(rc['control'],rc['limit'])+' '.join('%s:F%.2f/R%.2f%s'%(a['arm'][:2],a['full'],a['reduced'],'ok' if a['pass'] else 'X') for a in rc['arms'])+' shots=%.1f'%(s['shots_per_team_mean']))
line=f'{name} {json.dumps(over)} :: '+' | '.join(out)
print(line)
open(S+'/log.txt','a').write(line+'\n')
