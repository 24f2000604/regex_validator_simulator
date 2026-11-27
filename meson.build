project('nfagrep', 'c',
  version : '0.1.0',
  meson_version : '>=0.60',
  default_options : ['c_std=c11', 'warning_level=3', 'buildtype=release'])

src = files(
  'src/main.c',
  'src/nfa.c',
  'src/parse.c',
  'src/sim.c',
  'src/utf8.c',
  'src/util.c',
)

executable('nfagrep', src, install : true)
