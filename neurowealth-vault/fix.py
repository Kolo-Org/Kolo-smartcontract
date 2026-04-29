import os
import re

dir_path = 'contracts/vault/src/tests'
errors = 0
for filename in os.listdir(dir_path):
    if filename.endswith('.rs'):
        filepath = os.path.join(dir_path, filename)
        with open(filepath, 'r') as f:
            content = f.read()

        def fix_update(m):
            agent = m.group(1)
            amount = m.group(2)
            if amount.startswith('&(') and amount.endswith(')'):
                amount = amount[2:-1] # strip &( )
            elif amount.startswith('&'):
                amount = amount[1:] # strip &
            elif amount.startswith('(') and amount.endswith(')'):
                amount = amount[1:-1] # no & just parens
            return f'client.update_total_assets({agent}, &({amount}), &false, &0);'

        # We must re-add the `&` around the expression safely if they need them.
        # Wait, the compiler expects `&i128`. We were supplying `i128`.
        # So it must be `&({amount})`.
        # Example: if amount is `10_000`, new is `&(10_000)`. If `deposit + yield`, new is `&(deposit + yield)`.
        # Let's replace ONLY the known problematic ones in test_rounding_math.rs
        pass

# Instead of blindly replacing, I will just do exact targeted regex:
path = 'contracts/vault/src/tests/test_rounding_math.rs'
with open(path, 'r') as f: content = f.read()
content = re.sub(r'client\.update_total_assets\(&agent, &([^,]+)\s*\+\s*([^,]+),\s*&false\s*,\s*&0\);', r'client.update_total_assets(&agent, &(\1 + \2), &false, &0);', content)
content = re.sub(r'client\.update_total_assets\(&agent, &([^,]+)\s*\+\s*([^,]+)\s*\+\s*([^,]+)\s*\+\s*([^,]+),\s*&false\s*,\s*&0\);', r'client.update_total_assets(&agent, &(\1 + \2 + \3 + \4), &false, &0);', content)

with open(path, 'w') as f: f.write(content)

