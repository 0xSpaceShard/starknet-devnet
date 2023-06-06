import random

def generate(seed, n):
    random_generator = random.Random()
    random_generator.seed(seed)
    arr = []

    for _ in range(n):
        random_number = random_generator.getrandbits(128)
        arr.append(random_number)

    return arr