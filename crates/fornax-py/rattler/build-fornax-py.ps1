foreach ($whl in Get-ChildItem "$env:RECIPE_DIR/../dist/*.whl")
{
    pip install "$whl" -v
}
