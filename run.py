#!/usr/bin/python3

import subprocess
import os
import math
import sys


MULTITIME = os.path.join('multitime', 'multitime')
INTERPS = [{
    'name': 'luavm',
    'executable': os.path.join('target', 'release', 'luavm')
}, {
    'name': 'lua',
    'executable': os.path.join('lua', 'lua')
}, {
    'name': 'luajit',
    'executable': os.path.join('luajit', 'src', 'luajit')
}, {
    'name': 'luster',
    'executable': os.path.join('luster', 'target', 'release', 'luster')
}]


def confidence(std_dev, n):
    return 2.576 * std_dev / math.sqrt(n)


def run_bench(name, n, skip):
    means = []
    for interp in INTERPS:
        if interp['name'] in skip:
            print("Skipping {}".format(interp['name']))
            means.append((-1, -1))
            continue
        print('Running {} on {}'.format(interp['name'], name))
        # run benchmark `name` n times with multitime on a particular VM
        s = subprocess.run([MULTITIME, '-n', str(n),
                            interp["executable"], name],
                           stdout=subprocess.PIPE,
                           stderr=subprocess.STDOUT)
        # 4th line is the `real` running time
        result = s.stdout.decode()
        if 'Error' in result:
            # (-1, -1) means that the benchmark cannot be run
            print("Can't run {} with {}".format(name, interp['name']))
            means.append((-1, -1))
            continue
        result = result.split('\n')[3]
        # [`real`, `mean+-`, `std.dev`, `min`, `median`, `max`]
        cols = list(filter(lambda e: e != "", result.split(' ')))
        mean = float(cols[1].split('+')[0])
        std_dev = float(cols[2])
        conf = confidence(std_dev, n)
        print('Mean: {}, Std.Dev: {}, Confidence: {}'
                  .format(mean, std_dev, conf))
        means.append((mean, conf))
    table_row = ''
    for m in means:
        if m[0] == -1 or m[1] == -1:
            table_row += '&$-$ '
        else:
            table_row += '&${:.4f} \scriptstyle \pm \\small{{{:.4f}}}$ ' \
                .format(m[0], m[1])
    table_row += '\\\\'
    print(table_row)


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Process some integers.')
    parser.add_argument('--skip', metavar='N', type=str, nargs='*', default=[],
                        help='which VMs to skip, can be: '
                        'luavm, lua, luajit, and luster')
    parser.add_argument('-b', help='which benchmark to run')
    parser.add_argument('-n', type=int,
                        help='number of times to run benchmarks with each VM')
    args = parser.parse_args()
    run_bench(args.b, args.n, args.skip)
