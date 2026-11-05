import re
STRICT = re.compile(r"scope_budget\.py['\"]?\s+(init|add|revise|add-session|bind|cost-limit|close|listen|check)\b|usage_ledger\.py['\"]?\s+(tree|control|build)\b|--budget-record")
