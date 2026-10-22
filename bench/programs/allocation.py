total = 0
i = 0
while i < 200000:
    xs = [i, i + 1, i + 2]
    total += xs[0]
    i += 1
print(total)
