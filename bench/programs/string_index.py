s = "x" * 20000
total = 0
j = 0
while j < 2000:
    if s[j] == "x":
        total += 1
    j += 1
print(total)
