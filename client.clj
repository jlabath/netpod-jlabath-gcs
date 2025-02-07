#!/usr/bin/env bb

(require
 '[netpod.pods :as pods])


;;before running do
;;cargo build --release
;;to build the binary
(pods/with-pod "./target/debug/netpod-jlabath-gcs"
  ;; require is not suitable in macros
  ;; but one can also resolve things dynamically using resolve such as below
  (let [meta (resolve 'netpod.jlabath.gcs/meta)]
    (println @(meta "testerson"))))

