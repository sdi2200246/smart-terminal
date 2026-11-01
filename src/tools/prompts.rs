pub const ASK_USER_DESCRIPTION: &str = "Use this tool to achieve alignment with the user when their intent, \
  preferences, or context are unclear. Ask one focused question at a time. \
  Do not use this tool for information you can infer yourself.";

pub const BASH_DESCRIPTION: &str = "Execute a read-only shell command and return stdout, stderr, and exit code. \
    Use for investigation: grep, find, cat, head, tail, awk, sed, wc, sort, uniq, \
    git (log, diff, blame, show, branch), docker (ps, logs, inspect), \
    ps, lsof, df, du, env, uname, ping, curl (GET/HEAD only), dig, ss. \
    Destructive commands are blocked (rm, mv, cp, chmod, sudo, install, write redirects). \
    Output capped at 250 lines";

pub const DOCKER_DESCRIPTION: &str = "Get the current Docker state on this machine: running and stopped \
    containers (name, image, status, ports), whether the docker daemon is up, \
    and whether a docker-compose file exists in the current directory.";

pub const DOCKER_GUIDANCE: &str = "Call this tool if the user's intent involves Docker, Compose, containers, or names that act like containers (for example, 'restart db').";

pub const GIT_DIFF_STAGED_DESCRIPTION: &str =
    "Returns the staged changes (git diff --staged) of the current repository";

pub const GIT_DIFF_STAGED_GUIDANCE: &str = "Call this if the input is `git commit -m` (or similar) and a commit message must be generated. Read the staged diff first so the message is accurate.";

pub const READ_LAST_ERROR_DESCRIPTION: &str = "Read the stderr output of the user's last shell command,\
    Returns the last 200 characters at most. `truncated` indicates \
    whether earlier output was cut off. Empty content means the last \
    command produced no errors.";

pub const READ_LAST_ERROR_GUIDANCE: &str = "Read the captured stderr before answering when the user asks why a command failed, retries a likely failed command, or says a previously suggested command did not work. Base the correction on the captured error rather than guessing.";

pub const READ_DIR_DESCRIPTION: &str = "Read the contents of a directory. \
    Set recursive to true to include all subdirectories. \
    Automatically excludes target/ and .git/ directories. \
    Only for reading directory structure — not for reading file contents. \
    Returns an error if the path is a file;";

pub const READ_FILE_DESCRIPTION: &str = "Read lines from a file. Returns the content within the requested \
    line range and the total line count. Use start/end to window into large files. \
    Lines are 1-indexed and inclusive.";

