#!/usr/bin/python3

import subprocess


def run(name, bench, skip):
    cmd = ['python3', './run.py', '-b', bench, '-n', str(1)]
    if skip:
        cmd.append('--skip')
        cmd.extend(skip)
    s = subprocess.run(cmd, stdout=subprocess.PIPE)
    return '{}{}'.format(name, s.stdout.decode().split('\n')[-2])


table = """\\begin{{table}}[htbp]
  \\centering
  \\begin{{tabular}}{{@{{}}lp{{2.3cm}}p{{2.3cm}}p{{2.3cm}}p{{2.3cm}}@{{}}}}
    \\toprule
    & luavm & PUC-Rio Lua & LuaJIT & Luster
    \\\\
    \\midrule
    \\\\
    {fib}
    {fibi}
    {bintree}
    {nsieve}
    \\bottomrule
  \\end{{tabular}}
  \\caption{{Execution times (in seconds) of the VMs on four different benchmarks
    (reported with 99\% confidence intervals). Keys: `-': the benchmark cannot
    be run with a particular VM. Note that \\texttt{{luavm}} executes the
    \\texttt{{nsieve}} benchmark very slowly, thus its entry is measured in
    minutes.}}
  \\label{{table:bench}}
\\end{{table}}""".format(
    fib=run('fib(30)', './benchmarks/fib.lua', []),
    fibi=run('fib\_iter(60)', './benchmarks/fib_iter.lua', []),
    bintree=run('bin-trees', './benchmarks/binary-trees.lua', []),
    nsieve=run('nsieve', './benchmarks/nsieve.lua', ['luavm']),
)


with open('benchmark_table.tex', 'w') as f:
    f.write(table)
