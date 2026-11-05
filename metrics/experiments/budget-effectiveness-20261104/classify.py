import re
def otype(text):
    t = (text or '').lower()
    if re.search(r'\b(re)?review\b|rereview', t) and not re.search(r'implement|repair|qualif|integration', t): return 'review'
    if re.search(r'\btests?\b|test controls|negative control|diagnostic|\bcontrols? only|epoll dependency', t): return 'test'
    if re.search(r'qualif|\bactual\b|\blive\b|deploy|\breal\b|provision|rehearsal|observation and|pilot launch|\bjoin\b|native launch|runtime|falsify|admission-only|study', t): return 'live'
    if re.search(r'plan\b|planning|proposal|design|amendment|\bbudget\b|\bbid\b|prepar|readiness|inventory|selection|report|packet|spec\b|resolve original|interpretation|investigat|verify', t): return 'plan'
    if re.search(r'implement|repair|server|adapter|foundation|integration|\bfix|host|close boundary|engineering|candidate|merge|correct', t): return 'build'
    return 'other'
def outcome(r):
    if r.get('disposition'): return r['disposition']
    u = str(r.get('status') or '').upper()
    if not u or u == 'NONE': return 'unknown'
    if 'ACTIVE' in u or 'RUNNING' in u or 'AWAITING' in u: return 'open'
    bad = ('UNACCEPTED', 'UNFINISHED', 'FAILED', 'BUDGET_LIMIT', 'QUOTA_STOPPED', 'NO_VIABLE', 'CHANGES_REQUIRED', 'STOPPED', 'INCOMPLETE',
           'NO_SEND', 'UNAUTHORIZED', 'EOF_BEFORE', 'HELD_BEFORE_MODEL', 'NO_QUALIFIED', 'HOLD_UNREVIEWED', 'NO_JUSTIFIED', 'TEST_NOT_R', 'HELD_UNFINISHED', 'OWNER_DECISION_REQ', 'CAPACITY_CHECKPOINT_REQUEST')
    if any(k in u for k in bad): return 'not_accepted'
    if any(k in u for k in ('ACCEPT', 'COMPLETE', 'PASSED', 'BID_READY', 'DEPLOYED')): return 'accepted'
    return 'unknown'
