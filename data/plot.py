import matplotlib.pyplot as plt
import csv
import os

def plot_benchmark(benchmark):
    fig = plt.figure()
    ax = fig.subplots()
    for approach in sorted(os.listdir(benchmark)):
        approach = approach[:-4]
        with open(f"{benchmark}/{approach}.csv", 'r') as f:
            reader = csv.DictReader(f)
            x = []
            y = []
            for row in reader:
                x += [int(row['parallel'])]
                y += [(1 / (float(row['iterations']) / (float(row['duration_ns'])/1e9))) * 1e6]
            ax.plot(x, y, marker='o', label=approach)
    ax.set_xscale('log', base=2)
    ax.set_yscale('log')
    ax.set_ylim(ymin=1e-2, ymax=1e4)
    ax.set_ylabel('µs per iteration')
    ax.set_xlabel('parallelism')
    ax.set_title(benchmark)
    ax.legend()
    plt.savefig(f"{benchmark}.png")

for benchmark in sorted(os.listdir(".")):
    if not os.path.isdir(benchmark):
        continue
    plot_benchmark(benchmark)


