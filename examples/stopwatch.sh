#!/bin/bash

# This example is adapted (and simplified) from a real-world project that uses
# argparsh to build a timer that logs stats while the timer is active

PIDFILE=${TMPDIR:-/tmp}/stopwatch_pid

start_collection() {
  local -n args=$1
  echo "saving results to" ${args["outdir"]}
  mkdir -p ${args["outdir"]}
  while true; do
    free -h >> ${args["outdir"]}/memory.txt
    df -h >> ${args["outdir"]}/disk.txt
    sleep ${args["interval"]}
  done
}

stop_collection() {
  kill -SIGKILL $(cat $PIDFILE)
}

parser=$({
  argparsh new $0 -d "Stopwatch that collects stats while active"
  argparsh subparser_init --required true --metaname command

  argparsh subparser_add start
  argparsh set_defaults --subparser start --command start_collection
  argparsh subparser_add stop
  argparsh set_defaults --subparser stop --command stop_collection

  argparsh add_arg --subparser start "outdir"
  argparsh add_arg --subparser start -i --interval -- --type int --default 1
})
eval $(argparsh parse $parser --format assoc_array --name args_ -- "$@")

# Start the supplied command as a background process
${args_["command"]} args_ &

childpid=$!
if [ "${args_["command"]}" == "start_collection" ]; then
  # Save the background proc id
  echo "started collection with PID" $childpid
  echo $childpid > $PIDFILE
fi
