# mgba_gaming_agent

This repository is a work in progress, and the goal is to learn and experiment with various ML/deep learning modeling techniques to solve abstract problems.

As of right now, the application can be run in a few different ways:
1) Simply spawns a C client/server and allows you to run mgba directly with them, but this expects you to have the repo installed inside of mgba's repo
    -- mgba/src/platform/example/client-server/mgba_gaming_agent
2) Spawns a C server and runs a rust client to IO with it
    -- Same constraints as 1)
3) Spawns a rust server (running mgba natively via bindings) and runs a rust client to IO with it
4) Runs mgba core natively in a single thread

Those are the methods of operation with the package, however, there's an extra layer that can be done

Running with the argument Management Boss will spawn a bunch of subprocesses (called Workers in the code) that will in turn actually run the mgba cores.

Of most importance, the main application is assuming you have an `agent_config.json` in the root dir, you can copy over one of the templates (`configuration_templates`) and adjust accordingly

Right now that's pretty much it, no AI learning architecture yet, but that is what the next stage of development will really focus on.

You can run with --help for some of the variance that can be run in the application.
