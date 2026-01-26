class Dynamic
  def execute(code)
    # METAPROGRAMMING: DSL builder requires dynamic code execution
    eval(code)
  end
end
