# Alpha project targets

User scope: websites, applications and Java Minecraft plugins. The bridge is
language-neutral for file operations; tool execution remains runtime-specific.
An AI model's size does not grant additional capabilities or remove preconditions.

| Target | Common editing tools | Build/test delivery gate |
| --- | --- | --- |
| Website | list/search/read/write/patch, exact versions | Locally selected isolated web/Node test profile; preview separately scoped |
| Application | same project-relative editing | Operator-selected stack/runtime/profile, fixed executable/args, dependency and output scopes |
| Java Minecraft plugin | same editing for Java, resources and build descriptors | Explicit Java toolchain isolation eligibility, build profile, dependency access and generated artifact verification; disposable server test separately admitted |

Editing source is not the same as successfully compiling or running a project.
The first generated Windows editing proof uses HTML only; no Java build or
Minecraft server execution has yet been measured. Java/Gradle/Maven eligibility
must not be silently inferred from existing Node isolation. Minecraft target API,
game/server version and Java version are chosen for the test project when that
path is ready; no version or dependency download is guessed now.

For applications the target stack remains a local project choice, not a fixed
language imposed by Optic. Package/dependency acquisition, registries and caches
need explicit bounded network/read/write scope. Build scripts and tests are
repository code, not harmless fixed tools solely because they have a familiar
name. A plugin server may load arbitrary plugin code and open sockets; a real
Minecraft server test needs its own process/network grant and cleanup proof.

Delivery order: common FS editing first, existing isolated Node execution path
second, Java eligibility/build third, additional application runtimes thereafter.
This sequencing does not drop the Java/application goals or claim them ready.
No toolchain is installed, no package is downloaded and no server started by
this target specification. Single-session/shared-connection alpha remains.
