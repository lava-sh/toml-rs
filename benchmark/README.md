```bash
python benchmark/run.py                                                                                                                             
Parsing data.toml 10000 times:                                                                                                                        
------------------------------------------------------
    parser |  exec time | performance (more is better)
-----------+------------+-----------------------------
   toml_rs |    0.752 s
     rtoml |      1.1 s
  pytomlpp |     1.64 s
   tomllib |     6.08 s
      toml |     17.1 s
     qtoml |       17 s
   tomlkit |      137 s

Fastest parser: toml_rs (0.75236 s)

Performance relative to fastest parser:
   toml_rs | 100.00%
     rtoml | 68.61%
  pytomlpp | 45.93%
   tomllib | 12.37%
      toml | 4.40%
     qtoml | 4.43%
   tomlkit | 0.55%
```