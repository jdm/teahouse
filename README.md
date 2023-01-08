TODO:
* stairs/doors - collisions lead to other environments
* create general state machine component + events for updating it + systems for running in particular states
* make z-ordering occur based on y position sorting
  * system that runs each frame and sets z property based on tile position of each movable?
* player inventory and active item
* pathfinding support for multi-tile entities
* give up on pathfinding if it's taking too long (max # of attempts?)
* rewrite cat behaviour as scripted triggers
  * trigger 1: set local sleeping var, set sleep anim, wait random amount of time -> trigger 2
  * trigger 2: clear local sleeping var, move to random player/customer -> trigger 3
  * trigger 3: move to bed -> trigger 1
  * run actions: trigger 11
  * interaction trigger: conditional on local sleeping var, conditional on relationship
* rewrite kettle interaction as scripted trigger
* rewrite tea stash interaction as scripted trigger
* rewrite menu interaction as scripted trigger
* rewrite customer behaviour as scipted triggers
  * trigger 1: wait random amount of time -> trigger 2
  * trigger 2: spawn customer at door -> trigger 3
  * trigger 3: move customer to random chair, trigger 3 if failed, trigger 4
  * trigger 4: check for local var received tea -> trigger 5, wait 10s -> trigger 4
  * interaction trigger: conditional: ordering dialogue, receiving tea dialogue, set local var received tea
  * trigger 5: move customer to door, trigger 5 if failed, trigger 6
  * trigger 6: despawn customer
  how to persist affection in this model? perhaps don't use a component, only a resource?
* clean up local variables/triggers when despawning?
