xs = []
i = 0
while i < 1024:
    xs.append(i % 7)
    i += 1

def step(total, value):
    return total + value * 3

total = 0
round = 0
while round < 10000:
    for v in xs:
        total = step(total, v)
    round += 1
print(total)
