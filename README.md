TODO:
* stairs/doors - collisions lead to other environments
* create general state machine component + events for updating it + systems for running in particular states
  * rewrite customer states following cat state pattern
* make z-ordering occur based on y position sorting
  * system that runs each frame and sets z property based on tile position of each movable?
* player inventory and active item
* pathfinding support for multi-tile entities
* give up on pathfinding if it's taking too long (max # of attempts?)
* persistent customer relationships
