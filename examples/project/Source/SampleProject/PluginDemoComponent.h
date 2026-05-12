#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "Engine/Engine.h"
#include "PluginDemoComponent.generated.h"

/**
 * A component that demonstrates using NPM-distributed plugins in Blueprints
 */
UCLASS(ClassGroup=(Custom), meta=(BlueprintSpawnableComponent))
class SAMPLEPROJECT_API UPluginDemoComponent : public UActorComponent
{
    GENERATED_BODY()

public:
    UPluginDemoComponent();

protected:
    virtual void BeginPlay() override;

public:
    /** Get a random color using ExamplePlugin */
    UFUNCTION(BlueprintCallable, Category = "Plugin Demo")
    FLinearColor GetRandomColor();
    
    /** Format a number with commas using ExamplePlugin */
    UFUNCTION(BlueprintCallable, Category = "Plugin Demo")
    FString FormatNumber(int32 Number);
    
    /** Get a greeting message using ExamplePlugin */
    UFUNCTION(BlueprintCallable, Category = "Plugin Demo")
    FString GetGreeting(const FString& Name);
    
    /** Create a status message using DependencyPlugin */
    UFUNCTION(BlueprintCallable, Category = "Plugin Demo")
    FString CreateStatusMessage(const FString& Status, int32 Value);
    
    /** Get a random spawn location using DependencyPlugin */
    UFUNCTION(BlueprintCallable, Category = "Plugin Demo")
    FVector GetRandomSpawnLocation(FVector Center, float Radius);
    
    /** Generate a system report using DependencyPlugin */
    UFUNCTION(BlueprintCallable, Category = "Plugin Demo")
    FString GenerateReport();
    
    /** Demonstrate all plugin features */
    UFUNCTION(BlueprintCallable, Category = "Plugin Demo")
    void DemonstrateAllFeatures();

private:
    /** Check if plugins are available */
    bool ArePluginsAvailable() const;
};