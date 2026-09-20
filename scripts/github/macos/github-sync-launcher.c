#include <errno.h>
#include <spawn.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>

extern char **environ;

int main(int argc, char *argv[]) {
  if (argc != 2) {
    fprintf(stderr, "usage: github-sync-launcher RUNNER_PATH\n");
    return 64;
  }

  pid_t child = 0;
  char *child_argv[] = {"/bin/bash", argv[1], NULL};
  int spawn_result = posix_spawn(&child, "/bin/bash", NULL, NULL, child_argv, environ);
  if (spawn_result != 0) {
    fprintf(stderr, "launcher spawn failed: %s\n", strerror(spawn_result));
    return 70;
  }

  int status = 0;
  while (waitpid(child, &status, 0) == -1) {
    if (errno != EINTR) {
      fprintf(stderr, "launcher wait failed: %s\n", strerror(errno));
      return 70;
    }
  }

  if (WIFEXITED(status)) {
    return WEXITSTATUS(status);
  }
  if (WIFSIGNALED(status)) {
    return 128 + WTERMSIG(status);
  }
  return 70;
}
