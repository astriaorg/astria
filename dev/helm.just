_default:
  @just --list helm

render chart *args:
  helm template --debug --dry-run -name {{chart}} {{justfile_directory()}}/charts/{{chart}} -n debug {{args}} | yq

dependency command="update":
  #!/usr/bin/env bash
  for chart in $(readlink -qf {{justfile_directory()}}/charts/*/) ; do
    test -f $chart/Chart.yaml && helm dependency {{command}} $chart
  done
