import json, sys, subprocess, shutil, os, time
ROOT='C:/Users/jayte/Documents/dev/SoccerManager'
S='C:/Users/jayte/AppData/Local/Temp/claude/C--Users-jayte-Documents-dev-SoccerManager/a7ca6102-5eb6-4026-b025-47fc91bca540/scratchpad/tune'
BIN=os.environ.get('BIN', ROOT+'/target/release/engine-cli.exe')
name=sys.argv[1]; over=json.loads(sys.argv[2]); what=sys.argv[3] if len(sys.argv)>3 else 'red'
pm=int(sys.argv[4]) if len(sys.argv)>4 else 300
d=f'{S}/{name}'
if not os.path.exists(d+'/content'):
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
    return json.load(open(od+'/report.json'))
if 'red' in what:
    r=run(['--suite','red-card','--seed','1','--matches','120'],'red')
    rc=r['calib.red_card']; s=r['calib.suites']['red-card']
    out.append('RED ctrl=%s lim=%.2f '%(rc['control'],rc['limit'])+' '.join('%s:F%.2f/R%.2f%s'%(a['arm'][:2],a['full'],a['reduced'],'ok' if a['pass'] else 'X') for a in rc['arms'])+' fouls=%.1f shots=%.1f'%(s['fouls_per_team_mean'],s['shots_per_team_mean']))
if 'pairs' in what:
    r=run(['--suite','formations','--pairing','4-4-1-1 v 4-4-2','--pairing','3-4-3 v 4-4-2','--seed','42','--matches',str(pm)],'pairs')
    out.append('PAIRS '+' '.join('%s:%s'%('v'.join(f['pairing']),f['goals_for_mean']) for f in r['calib.formations'])+' fouls=%.1f shots=%.1f yc=%.2f so=%.3f'%(r['calib.suites']['formations']['fouls_per_team_mean'],r['calib.suites']['formations']['shots_per_team_mean'],r['calib.suites']['formations']['yellow_cards_per_team_mean'],r['calib.suites']['formations']['sending_off_share']))
line=f'{name} {json.dumps(over)} :: '+' | '.join(out)
print(line)
open(S+'/log.txt','a').write(line+'\n')
