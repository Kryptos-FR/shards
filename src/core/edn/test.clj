;; (def Root (Node))
;; (def callme (fn [] Root))
;; (callme)

(def call2 (fn [a b c] b))
(call2 0 1 2)

(do
  (call2 2 3 4)
  '(call2 5 5 5)
  )

(let [a 10 b [3 4 11]] (first b))
("Hello world")