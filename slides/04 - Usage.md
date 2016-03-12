## CLI
`$ ws-p2p --help`

## Connecting peers locally
```zsh
$ ws-p2p --server localhost:3013 ws://localhost:3012
$ ws-p2p --server localhost:3014 ws://localhost:3012
$ ws-p2p --server localhost:3015 ws://localhost:3012 ws://localhost:3013

```

Network structure
-----------------

```
 3013 ---- 3012 ---- 3014
   \        |
    \       |
     \      |
      \     |
       \    |
        \   |
         \  |
          3015

```