def step(n):
    return n + 1
total = 0
i = 0
while i < 10000000:
    total = step(total)
    i += 1
print(total)
