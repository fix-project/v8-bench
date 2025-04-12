#include <criu/criu.h>
#include <stdlib.h>
#include <stdio.h>
#include <time.h>
#include <unistd.h>
#include <sys/wait.h>
#include <fcntl.h>
#include <sys/stat.h>

double gettime() {
  struct timespec now={0,0};
  clock_gettime(CLOCK_MONOTONIC, &now);
  return (double)now.tv_sec + 1.0e-9*now.tv_nsec;
}

int main(int argc, char **argv) {
  fprintf(stderr, "parent PID is %d\n", getpid());

  if (argc != 3) {
    fprintf(stderr,"Usage: %s <bytes of ram to malloc> <duration to run in seconds>\n", argv[0]);
    exit(1);
  }
  long long bytes = atoll(argv[1]);
  double seconds = atof(argv[2]);
  fprintf(stderr, "running for %f seconds with %lld bytes of malloced memory\n", seconds, bytes);

  char template[] = "/tmp/criu-test-XXXXXX";
  char *dir = mkdtemp(template);
  chmod(dir, 0777);
  int fd = open(dir, O_DIRECTORY);

  char *buf = malloc(bytes);
  fprintf(stderr, "initializing memory...\n");
  for (int i = 0; i < bytes; i++) {
    buf[i] = 1;
  }
  fprintf(stderr, "done!\n");

  criu_init_opts();
  criu_set_shell_job(true);
  criu_set_images_dir_fd(fd);
  fprintf(stderr, "forking\n");
  fflush(stdout);
  int pid = fork();
  if (pid == 0) {
    while (1) {
      criu_dump();
    }
  }

  waitpid(pid, NULL, 0);

  double start = gettime();
  // warm up for 1 second
  while (1) {
    int pid = criu_restore();
    while (kill(pid, 0) != -1) {}
    if (gettime() > start + 1) {
      break;
    }
  }

  start = gettime();
  int iterations = 0;
  while (1) {
    int pid = criu_restore();
    while (kill(pid, 0) != -1) {}
    if (gettime() > start + seconds) {
      break;
    }
    iterations++;
  }

  fprintf(stderr, "iterations per second: %f\n", iterations/seconds);
  fprintf(stderr, "µs per iteration: %f\n", seconds/iterations * 1e6);
  printf("%lld, %d, %f\n", bytes, iterations, seconds);

  fprintf(stderr, "You may want to clean up the directory \"%s\".\n", dir);

  exit(0);
}
