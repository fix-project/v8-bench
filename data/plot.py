import matplotlib.pyplot as plt
import csv
import os

def plot_benchmark(benchmark):
    fig = plt.figure()
    ax = fig.subplots()
    print(benchmark)
    for approach in sorted(os.listdir(benchmark)):
        approach = approach[:-4]
        print(f"{benchmark}/{approach}")
        with open(f"{benchmark}/{approach}.csv", 'r') as f:
            reader = csv.DictReader(f)
            data = {}
            for row in reader:
                iterations = float(row['iterations'])
                duration_s = float(row['duration_ns'])/1e9
                parallel = int(row['parallel'])
                if parallel not in data:
                    data[parallel] = []
                data[parallel] += [(iterations, duration_s)]
            X = []
            Y = []
            for n in data:
                ys = data[n]
                iterations = sum(y[0] for y in ys)
                time = ys[0][1]
                rate = time / iterations * 1e6
                X += [n]
                Y += [rate]
            ax.plot(X, Y, marker='o', label=approach)
    ax.set_xscale('log', base=2)
    ax.set_yscale('log')
    # ax.set_ylim(ymin=1e-2, ymax=1e4)
    ax.set_ylabel('µS per iteration')
    ax.set_xlabel('parallelism')
    ax.set_title(benchmark)
    ax.legend()
    plt.savefig(f"{benchmark}.png")

for benchmark in sorted(os.listdir(".")):
    if not os.path.isdir(benchmark):
        continue
    plot_benchmark(benchmark)


