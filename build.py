#!/usr/bin/python3

import os
import subprocess


def clone(url, name):
    code = 0
    if not os.path.isdir(name):
        p = subprocess.run(['git', 'clone', url, name])
        code = p.returncode
    return code


def make(name):
    rc = subprocess.run(['make'], cwd=name).returncode
    return rc


def cargo_build(name):
    return subprocess.run(['cargo', 'build', '--release'],
                          cwd=name).returncode

def build_multitime(name):
    commands = [['make', '-f', 'Makefile.bootstrap'],
                ['sh', '-c', './configure'],
                ['make']]
    rc = 0
    for cmd in commands:
        rc = rc or subprocess.run(cmd, cwd=name).returncode
    return rc

if __name__ == '__main__':
    to_build = [{
        'name': 'multitime',
        'url': 'https://github.com/ltratt/multitime',
        'type': [build_multitime],
    }, {
        'name': 'lua',
        'url': 'https://github.com/lua/lua',
        'type': [make],
    }, {
        'name': 'luajit',
        'url': 'http://luajit.org/git/luajit-2.0.git',
        'type': [make],
    }, {
        'name': 'luavm',
        'url': 'https://github.com/rbartlensky/Lua-interpreter',
        'type': [cargo_build],
    }, {
        'name': 'luster',
        'url': 'https://github.com/kyren/luster',
        'type': [cargo_build],
    }]
    for build in to_build:
        if clone(build['url'], build['name']) != 0:
            print('Failed to clone {}'.format(build['name']))
            break
        else:
            err = False
            for cb in build['type']:
                if cb(build['name']) != 0:
                    print('Failed to build {}'.format(build['name']))
                    err = True
                    break
            if err:
                break
        print('Finished building {}'.format(build['name']))
