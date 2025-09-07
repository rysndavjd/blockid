#!/bin/bash

if ! command -v gzip > /dev/null ; then 
    echo "Gzip is needed to extract test headers"
    exit 1
fi

cp ../headers/* ./

gunzip ./*.gz