#!/usr/bin/python3
"""GTK 통합 테스트 전용 CLI. 실제 계정·VPN·네트워크를 변경하지 않습니다."""
import json, os, sys, time
from pathlib import Path
path=Path(os.environ['ADGUARD_GUI_FIXTURE_STATE'])
state=json.loads(path.read_text())
args=sys.argv[1:]
state.setdefault('calls', []).append(args)
if args==['license']:
    case=state.get('license_case','stdout')
    path.write_text(json.dumps(state))
    if case=='timeout':
        time.sleep(60)
    if case=='error':
        print('Network unavailable',file=sys.stderr); sys.exit(2)
    if case in ('signedout', 'signedout_error'):
        print('Please log in to view your license info',file=sys.stderr); sys.exit(1 if case=='signedout_error' else 0)
    if case=='unknown':
        print('Unexpected output'); sys.exit(0)
    print('Logged in as --\nYou are using the PREMIUM version\nUp to 10 devices simultaneously\nYour subscription is valid until 2031-08-31',file=sys.stderr if case=='stderr' else sys.stdout)
elif args[:1]==['site-exclusions']:
    if args[1]=='mode':
        if len(args)==3: state['mode']=args[2]
        print('Current exclusion mode is '+state['mode'])
    else:
        mode=args[args.index('--for-mode')+1]
        items=state.setdefault(mode,[])
        if args[1]=='add': items.append(args[-1])
        if args[1]=='remove': items.remove(args[-1])
        print('Exclusions for '+mode+' mode:')
        print('\n'.join(items))
elif args==['status']: print('VPN is not connected')
elif args==['config','show']: print('Protocol: auto\nDNS upstream: default')
elif args==['list-locations']: print('KR  South Korea  Seoul  15 ms')
else:
    print('Unexpected command: '+repr(args),file=sys.stderr); sys.exit(3)
path.write_text(json.dumps(state))
