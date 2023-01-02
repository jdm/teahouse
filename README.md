TODO:
* stairs/doors - collisions lead to other environments
* create general state machine component + events for updating it + systems for running in particular states
  * rewrite customer states following cat state pattern
* make z-ordering occur based on y position sorting
  * system that runs each frame and sets z property based on tile position of each movable?
* spawn teapots on adjacent prop entity
* player inventory and active item
* move animation frame data into animation state values
* generalize facing direction animation state updates
* split spawn_sprite into events to spawn each kind of entity
* pathfinding support for multi-tile entities
* give up on pathfinding if it's taking too long (max # of attempts?)
