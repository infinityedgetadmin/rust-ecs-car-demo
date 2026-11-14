# assets/models

Drop-in location for pipeline-produced meshes. On startup the game calls
`Mesh::load_from_file("assets/models/car.glb")`; in v0.1 that loader is a stub, so the
procedural car is always used. No binary assets are committed in v0.1.
