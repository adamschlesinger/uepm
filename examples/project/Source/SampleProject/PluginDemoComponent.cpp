#include "PluginDemoComponent.h"
#include "ExamplePlugin.h"
#include "DependencyPlugin.h"

UPluginDemoComponent::UPluginDemoComponent()
{
    PrimaryComponentTick.bCanEverTick = false;
}

void UPluginDemoComponent::BeginPlay()
{
    Super::BeginPlay();
    
    UE_LOG(LogTemp, Warning, TEXT("PluginDemoComponent: Starting up"));
    
    if (ArePluginsAvailable())
    {
        UE_LOG(LogTemp, Log, TEXT("PluginDemoComponent: All NPM plugins are available!"));
        
        // Automatically demonstrate features when the component starts
        DemonstrateAllFeatures();
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("PluginDemoComponent: Some NPM plugins are missing!"));
    }
}

FLinearColor UPluginDemoComponent::GetRandomColor()
{
    if (FExamplePluginModule::IsAvailable())
    {
        return FExamplePluginUtils::GetRandomColor();
    }
    
    UE_LOG(LogTemp, Warning, TEXT("ExamplePlugin not available, returning default color"));
    return FLinearColor::White;
}

FString UPluginDemoComponent::FormatNumber(int32 Number)
{
    if (FExamplePluginModule::IsAvailable())
    {
        return FExamplePluginUtils::FormatNumberWithCommas(Number);
    }
    
    UE_LOG(LogTemp, Warning, TEXT("ExamplePlugin not available, returning unformatted number"));
    return FString::FromInt(Number);
}

FString UPluginDemoComponent::GetGreeting(const FString& Name)
{
    if (FExamplePluginModule::IsAvailable())
    {
        return FExamplePluginUtils::GetGreetingMessage(Name);
    }
    
    UE_LOG(LogTemp, Warning, TEXT("ExamplePlugin not available, returning simple greeting"));
    return FString::Printf(TEXT("Hello %s!"), *Name);
}

FString UPluginDemoComponent::CreateStatusMessage(const FString& Status, int32 Value)
{
    if (FDependencyPluginModule::IsAvailable())
    {
        return FDependencyPluginFeatures::CreateColorfulStatusMessage(Status, Value);
    }
    
    UE_LOG(LogTemp, Warning, TEXT("DependencyPlugin not available, returning simple status"));
    return FString::Printf(TEXT("Status: %s, Value: %d"), *Status, Value);
}

FVector UPluginDemoComponent::GetRandomSpawnLocation(FVector Center, float Radius)
{
    if (FDependencyPluginModule::IsAvailable())
    {
        return FDependencyPluginFeatures::GetRandomSpawnLocation(Center, Radius);
    }
    
    UE_LOG(LogTemp, Warning, TEXT("DependencyPlugin not available, returning center location"));
    return Center;
}

FString UPluginDemoComponent::GenerateReport()
{
    if (FDependencyPluginModule::IsAvailable())
    {
        return FDependencyPluginFeatures::GenerateSystemReport();
    }
    
    UE_LOG(LogTemp, Warning, TEXT("DependencyPlugin not available, returning simple report"));
    return TEXT("System Report: Plugins not available");
}

void UPluginDemoComponent::DemonstrateAllFeatures()
{
    UE_LOG(LogTemp, Warning, TEXT("=== PluginDemoComponent Feature Demo ==="));
    
    // Test ExamplePlugin features
    FLinearColor Color = GetRandomColor();
    UE_LOG(LogTemp, Log, TEXT("Random Color: R=%.2f G=%.2f B=%.2f"), Color.R, Color.G, Color.B);
    
    FString FormattedNumber = FormatNumber(1234567);
    UE_LOG(LogTemp, Log, TEXT("Formatted Number: %s"), *FormattedNumber);
    
    FString Greeting = GetGreeting(TEXT("Blueprint User"));
    UE_LOG(LogTemp, Log, TEXT("Greeting: %s"), *Greeting);
    
    // Test DependencyPlugin features
    FString StatusMsg = CreateStatusMessage(TEXT("BLUEPRINT_READY"), 42);
    UE_LOG(LogTemp, Log, TEXT("Status Message: %s"), *StatusMsg);
    
    FVector SpawnLoc = GetRandomSpawnLocation(FVector(0, 0, 0), 500.0f);
    UE_LOG(LogTemp, Log, TEXT("Random Spawn Location: %s"), *SpawnLoc.ToString());
    
    FString Report = GenerateReport();
    UE_LOG(LogTemp, Log, TEXT("System Report:\n%s"), *Report);
    
    UE_LOG(LogTemp, Warning, TEXT("=== Feature Demo Complete ==="));
}

bool UPluginDemoComponent::ArePluginsAvailable() const
{
    return FExamplePluginModule::IsAvailable() && FDependencyPluginModule::IsAvailable();
}